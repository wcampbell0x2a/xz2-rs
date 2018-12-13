#![allow(bad_style)]

extern crate libc;
extern crate lzma_sys;

use libc::*;
use lzma_sys::*;

include!(concat!(env!("OUT_DIR"), "/all.rs"));
