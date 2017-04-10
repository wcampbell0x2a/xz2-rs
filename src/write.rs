//! Writer-based compression/decompression streams

use std::io::prelude::*;
use std::io;

use stream::{Action, Status, Stream, Check};

/// A compression stream which will have uncompressed data written to it and
/// will write compressed data to an output stream.
pub struct XzEncoder<W: Write> {
    data: Stream,
    obj: Option<W>,
    buf: Vec<u8>,
}

/// A compression stream which will have compressed data written to it and
/// will write uncompressed data to an output stream.
pub struct XzDecoder<W: Write> {
    data: Stream,
    obj: Option<W>,
    buf: Vec<u8>,
}

impl<W: Write> XzEncoder<W> {
    /// Create a new compression stream which will compress at the given level
    /// to write compress output to the give output stream.
    pub fn new(obj: W, level: u32) -> XzEncoder<W> {
        let stream = Stream::new_easy_encoder(level, Check::Crc64).unwrap();
        XzEncoder::new_stream(obj, stream)
    }

    /// Create a new encoder which will use the specified `Stream` to encode
    /// (compress) data into the provided `obj`.
    pub fn new_stream(obj: W, stream: Stream) -> XzEncoder<W> {
        XzEncoder {
            data: stream,
            obj: Some(obj),
            buf: Vec::with_capacity(32 * 1024),
        }
    }

    fn dump(&mut self) -> io::Result<()> {
        if self.buf.len() > 0 {
            try!(self.obj.as_mut().unwrap().write_all(&self.buf));
            self.buf.truncate(0);
        }
        Ok(())
    }

    fn do_finish(&mut self) -> io::Result<()> {
        loop {
            try!(self.dump());
            let res = try!(self.data.process_vec(&[], &mut self.buf, Action::Finish));
            if res == Status::StreamEnd {
                break
            }
        }
        self.dump()
    }

    /// Consumes this encoder, flushing the output stream.
    ///
    /// This will flush the underlying data stream and then return the contained
    /// writer if the flush succeeded.
    pub fn finish(mut self) -> io::Result<W> {
        try!(self.do_finish());
        Ok(self.obj.take().unwrap())
    }

    /// Returns the number of bytes produced by the compressor
    ///
    /// Note that, due to buffering, this only bears any relation to
    /// `total_in()` after a call to `flush()`.  At that point,
    /// `total_out() / total_in()` is the compression ratio.
    pub fn total_out(&self) -> u64 {
        self.data.total_out()
    }

    /// Returns the number of bytes consumed by the compressor
    /// (e.g. the number of bytes written to this stream.)
    pub fn total_in(&self) -> u64 {
        self.data.total_in()
    }
}

impl<W: Write> Write for XzEncoder<W> {
    fn write(&mut self, data: &[u8]) -> io::Result<usize> {
        loop {
            try!(self.dump());

            let total_in = self.total_in();
            self.data.process_vec(data, &mut self.buf, Action::Run)
                .unwrap();
            let written = (self.total_in() - total_in) as usize;

            if written > 0 || data.len() == 0 {
                return Ok(written)
            }
        }
    }

    fn flush(&mut self) -> io::Result<()> {
        loop {
            try!(self.dump());
            let status = self.data.process_vec(&[], &mut self.buf,
                                               Action::FullFlush).unwrap();
            if status == Status::StreamEnd {
                break
            }
        }
        self.obj.as_mut().unwrap().flush()
    }
}

impl<W: Write> Drop for XzEncoder<W> {
    fn drop(&mut self) {
        if self.obj.is_some() {
            let _ = self.do_finish();
        }
    }
}

impl<W: Write> XzDecoder<W> {
    /// Creates a new decoding stream which will decode all input written to it
    /// into `obj`.
    pub fn new(obj: W) -> XzDecoder<W> {
        let stream = Stream::new_stream_decoder(u64::max_value(), 0).unwrap();
        XzDecoder::new_stream(obj, stream)
    }

    /// Creates a new decoding stream which will decode all input written to it
    /// into `obj`.
    ///
    /// A custom `stream` can be specified to configure what format this decoder
    /// will recognize or configure other various decoding options.
    pub fn new_stream(obj: W, stream: Stream) -> XzDecoder<W> {
        XzDecoder {
            data: stream,
            obj: Some(obj),
            buf: Vec::with_capacity(32 * 1024),
        }
    }

    fn dump(&mut self) -> io::Result<()> {
        if self.buf.len() > 0 {
            try!(self.obj.as_mut().unwrap().write_all(&self.buf));
            self.buf.truncate(0);
        }
        Ok(())
    }

    fn do_finish(&mut self) -> io::Result<()> {
        loop {
            try!(self.dump());
            let res = try!(self.data.process_vec(&[], &mut self.buf,
                                                 Action::Run));

            // When decoding a truncated file, XZ returns LZMA_BUF_ERROR and
            // decodes no new data, which corresponds to this crate's MemNeeded
            // status.  Since we're finishing, we cannot provide more data so
            // this is an error.
            //
            // See the 02_decompress.c example in xz-utils.
            if self.buf.is_empty() && res == Status::MemNeeded {
                let msg = "xz compressed stream is truncated or otherwise corrupt";
                return Err(io::Error::new(io::ErrorKind::UnexpectedEof, msg))
            }

            if res == Status::StreamEnd {
                break
            }

        }
        self.dump()
    }

    /// Unwrap the underlying writer, finishing the compression stream.
    pub fn finish(&mut self) -> io::Result<W> {
        try!(self.do_finish());
        Ok(self.obj.take().unwrap())
    }

    /// Returns the number of bytes produced by the decompressor
    ///
    /// Note that, due to buffering, this only bears any relation to
    /// `total_in()` after a call to `flush()`.  At that point,
    /// `total_in() / total_out()` is the compression ratio.
    pub fn total_out(&self) -> u64 {
        self.data.total_out()
    }

    /// Returns the number of bytes consumed by the decompressor
    /// (e.g. the number of bytes written to this stream.)
    pub fn total_in(&self) -> u64 {
        self.data.total_in()
    }
}

impl<W: Write> Write for XzDecoder<W> {
    fn write(&mut self, data: &[u8]) -> io::Result<usize> {
        loop {
            try!(self.dump());

            let before = self.total_in();
            let res = try!(self.data.process_vec(data, &mut self.buf,
                                                 Action::Run));
            let written = (self.total_in() - before) as usize;

            if written > 0 || data.len() == 0 || res == Status::StreamEnd {
                return Ok(written)
            }
        }
    }

    fn flush(&mut self) -> io::Result<()> {
        try!(self.dump());
        self.obj.as_mut().unwrap().flush()
    }
}

impl<W: Write> Drop for XzDecoder<W> {
    fn drop(&mut self) {
        if self.obj.is_some() {
            let _ = self.do_finish();
        }
    }
}

#[cfg(test)]
mod tests {
    use std::io::prelude::*;
    use std::iter::repeat;
    use super::{XzEncoder, XzDecoder};

    #[test]
    fn smoke() {
        let d = XzDecoder::new(Vec::new());
        let mut c = XzEncoder::new(d, 6);
        c.write_all(b"12834").unwrap();
        let s = repeat("12345").take(100000).collect::<String>();
        c.write_all(s.as_bytes()).unwrap();
        let data = c.finish().unwrap().finish().unwrap();
        assert_eq!(&data[0..5], b"12834");
        assert_eq!(data.len(), 500005);
        assert!(format!("12834{}", s).as_bytes() == &*data);
    }

    #[test]
    fn write_empty() {
        let d = XzDecoder::new(Vec::new());
        let mut c = XzEncoder::new(d, 6);
        c.write(b"").unwrap();
        let data = c.finish().unwrap().finish().unwrap();
        assert_eq!(&data[..], b"");
    }

    #[test]
    fn qc() {
        ::quickcheck::quickcheck(test as fn(_) -> _);

        fn test(v: Vec<u8>) -> bool {
            let w = XzDecoder::new(Vec::new());
            let mut w = XzEncoder::new(w, 6);
            w.write_all(&v).unwrap();
            v == w.finish().unwrap().finish().unwrap()
        }
    }
}