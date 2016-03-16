extern crate ctest;

use std::env;

fn main() {
    let out = env::var("DEP_LZMA_INCLUDE").unwrap();
    let mut cfg = ctest::TestGenerator::new();

    cfg.header("lzma.h");
    cfg.include(&out);
    cfg.type_name(|n, _s| n.to_string());
    cfg.generate("../lzma-sys/src/lib.rs", "all.rs");
}