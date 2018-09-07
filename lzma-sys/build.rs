extern crate cc;
extern crate filetime;
extern crate pkg_config;

use std::env;
use std::ffi::OsString;
use std::fs;
use std::path::{PathBuf, Path};
use std::process::Command;

use filetime::FileTime;

use cc::windows_registry::VsVers;

static DEFINES: &[&str] = &[
    "HAVE_STDINT_H",
    "HAVE_STDBOOL_H",
    "HAVE_STRING_H",
    "HAVE_DECODER_LZMA1",
    "HAVE_DECODER_LZMA2",
    "HAVE_ENCODER_LZMA1",
    "HAVE_ENCODER_LZMA2",
    "HAVE_DECODER_LZ",
    "HAVE_ENCODER_LZ",
    "HAVE_DECODER_DELTA",
    "HAVE_ENCODER_DELTA",
    "HAVE_DECODER_SPARC",
    "HAVE_ENCODER_SPARC",
    "HAVE_DECODER_X86",
    "HAVE_ENCODER_X86",
    "HAVE_CHECK_SHA256",
    "HAVE_CHECK_CRC64",
    "HAVE_CHECK_CRC32",
    "HAVE_MF_BT2",
    "HAVE_MF_BT3",
    "HAVE_MF_BT4",
    "HAVE_MF_HC3",
    "HAVE_MF_HC4",
];

static SKIP_FILENAMES: &[&str] = &[
    "crc32_small", "crc64_small"
];

fn main() {
    let target = env::var("TARGET").unwrap();

    println!("cargo:rerun-if-env-changed=LZMA_API_STATIC");
    let want_static = env::var("LZMA_API_STATIC").is_ok();
    if !want_static && pkg_config::probe_library("liblzma").is_ok() {
        return;
    }

    let include_dir = env::current_dir().unwrap().join("xz-5.2.3/src/liblzma/api");
    println!("cargo:include={}", include_dir.display());

    let features = env::var("CARGO_CFG_TARGET_FEATURE")
                        .unwrap_or(String::new());

    let src_files = [
        "xz-5.2.3/src/liblzma/common",
        "xz-5.2.3/src/liblzma/lzma",
        "xz-5.2.3/src/liblzma/lz",
        "xz-5.2.3/src/liblzma/check",
        "xz-5.2.3/src/liblzma/delta",
        "xz-5.2.3/src/liblzma/rangecoder",
        "xz-5.2.3/src/liblzma/simple",
    ].iter().flat_map(|dir| read_dir_files(dir)).chain(vec![
        "xz-5.2.3/src/common/tuklib_cpucores.c".into(),
        "xz-5.2.3/src/common/tuklib_physmem.c".into()
    ]);

    let mut build = cc::Build::new();

    for define in DEFINES {
        build.define(define, "1");
    }

    build.files(src_files)
        .include("xz-5.2.3/src/liblzma/api")
        .include("xz-5.2.3/src/liblzma/lzma")
        .include("xz-5.2.3/src/liblzma/lz")
        .include("xz-5.2.3/src/liblzma/check")
        .include("xz-5.2.3/src/liblzma/simple")
        .include("xz-5.2.3/src/liblzma/delta")
        .include("xz-5.2.3/src/liblzma/common")
        .include("xz-5.2.3/src/liblzma/rangecoder")
        .include("xz-5.2.3/src/common");

    if target.ends_with("msvc") {
        build.define("MYTHREAD_VISTA", "1");
    } else {
        build.define("_POSIX_C_SOURCE", "199506L")
            .define("MYTHREAD_POSIX", "1")
            .flag("-std=c99")
            .flag("-pthread");
    }

    build.compile("liblzma.a");
}

fn read_dir_files(dir: &str) -> impl Iterator<Item=PathBuf> {
    fs::read_dir(dir)
        .expect(&format!("failed to read dir {}", dir))
        .filter_map(|ent| {
            let ent = ent.expect("failed to read entry");

            if ent.file_type().unwrap().is_dir() {
                return None;
            }

            let path = ent.path();

            if path.extension().unwrap() != "c" {
                return None;
            }

            {
                let file_stem = path.file_stem().unwrap().to_str().unwrap();
                if SKIP_FILENAMES.contains(&file_stem) { return None }
                if file_stem.ends_with("tablegen") { return None }
            }

            Some(path)
        })
}
