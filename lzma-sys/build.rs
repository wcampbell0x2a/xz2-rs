extern crate gcc;

use std::env;
use std::ffi::OsString;
use std::fs;
use std::path::{PathBuf, Path};
use std::process::Command;

macro_rules! t {
    ($e:expr) => (match $e {
        Ok(t) => t,
        Err(e) => panic!("{} return the error {}", stringify!($e), e),
    })
}

fn main() {
    let target = env::var("TARGET").unwrap();
    let dst = PathBuf::from(env::var_os("OUT_DIR").unwrap());
    let src = t!(env::current_dir());

    println!("cargo:rustc-link-search={}/lib", dst.display());
    println!("cargo:root={}", dst.display());
    println!("cargo:include={}/include", dst.display());
    println!("cargo:rerun-if-changed=xz-5.2.2/configure");

    if target.contains("msvc") {
        println!("cargo:rustc-link-lib=static=liblzma");
        let mut msbuild = gcc::windows_registry::find(&target, "msbuild")
                              .expect("needs msbuild installed");
        let build = dst.join("build");
        cp_r(Path::new("xz-5.2.2"), &build);

        run(msbuild.current_dir(build.join("windows"))
                   .arg("/p:Configuration=Release"));
        t!(fs::create_dir(dst.join("lib")));
        t!(fs::create_dir(dst.join("include")));
        t!(fs::copy(build.join("windows/Release/Win32/liblzma.lib"),
                    dst.join("lib/liblzma.lib")));
        t!(fs::copy(build.join("src/liblzma/api/lzma.h"),
                    dst.join("include/lzma.h")));
        cp_r(&build.join("src/liblzma/api/lzma"), &dst.join("include/lzma"));
    } else {
        println!("cargo:rustc-link-lib=static=lzma");
        let cfg = gcc::Config::new();
        let compiler = cfg.get_compiler();

        let _ = fs::create_dir(&dst.join("build"));

        let mut cmd = Command::new("sh");
        let mut cflags = OsString::new();
        for arg in compiler.args() {
            cflags.push(arg);
            cflags.push(" ");
        }
        cmd.env("CC", compiler.path())
           .env("CFLAGS", cflags)
           .current_dir(&dst.join("build"))
           .arg(src.join("xz-5.2.2/configure").to_str().unwrap()
                   .replace("C:\\", "/c/")
                   .replace("\\", "/"));
        cmd.arg(format!("--prefix={}", dst.display()));
        cmd.arg("--disable-doc");
        cmd.arg("--disable-lzma-links");
        cmd.arg("--disable-lzmainfo");
        cmd.arg("--disable-lzmadec");
        cmd.arg("--disable-xz");
        cmd.arg("--disable-xzdec");
        cmd.arg("--disable-shared");
        cmd.arg("--disable-nls");
        cmd.arg("--disable-rpath");
        cmd.arg("--enable-threads=yes");

        run(&mut cmd);
        run(Command::new("make")
                    .arg(&format!("-j{}", env::var("NUM_JOBS").unwrap()))
                    .arg("install")
                    .current_dir(&dst.join("build/src/liblzma")));
    }
}

fn run(cmd: &mut Command) {
    println!("running: {:?}", cmd);
    assert!(t!(cmd.status()).success());
}

fn cp_r(src: &Path, dst: &Path) {
    t!(fs::create_dir(dst));
    for e in t!(src.read_dir()).map(|e| t!(e)) {
        let src = e.path();
        let dst = dst.join(e.file_name());
        if t!(e.file_type()).is_dir() {
            cp_r(&src, &dst);
        } else {
            t!(fs::copy(&src, &dst));
        }
    }
}
