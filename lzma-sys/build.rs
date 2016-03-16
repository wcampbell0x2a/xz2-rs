extern crate gcc;

use std::env;
use std::ffi::OsString;
use std::fs;
use std::path::PathBuf;
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
    println!("cargo:rustc-link-lib=static=lzma");
    println!("cargo:root={}", dst.display());
    println!("cargo:include={}/include", dst.display());
    println!("cargo:rerun-if-changed=xz-5.2.2/configure");

    if target.contains("msvc") {
        panic!("msvc");
    } else {
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

// fn build_msvc(target: &str) {
//     let cmd = gcc::windows_registry::find(target, "nmake.exe");
//     let mut cmd = cmd.unwrap_or(Command::new("nmake.exe"));
//     let src = env::current_dir().unwrap();
//     let dst = PathBuf::from(env::var_os("OUT_DIR").unwrap());
//     let machine = if target.starts_with("x86_64") {
//         "x64"
//     } else if target.starts_with("i686") {
//         "x86"
//     } else {
//         panic!("unknown msvc target: {}", target);
//     };
//
//     t!(fs::create_dir_all(dst.join("include/curl")));
//     t!(fs::create_dir_all(dst.join("lib")));
//
//     cmd.current_dir(src.join("curl/winbuild"));
//     cmd.arg("/f").arg("Makefile.vc")
//        .arg("MODE=static")
//        .arg("ENABLE_IDN=yes")
//        .arg("DEBUG=no")
//        .arg("GEN_PDB=no")
//        .arg("ENABLE_WINSSL=yes")
//        .arg("ENABLE_SSPI=yes")
//        .arg(format!("MACHINE={}", machine));
//
//     if let Some(inc) = env::var_os("DEP_Z_ROOT") {
//         let inc = PathBuf::from(inc);
//         let mut s = OsString::from("WITH_DEVEL=");
//         s.push(&inc);
//         cmd.arg("WITH_ZLIB=static").arg(s);
//
//         // the build system for curl expects this library to be called
//         // zlib_a.lib, so make sure it's named correctly (where libz-sys just
//         // produces zlib.lib)
//         let _ = fs::remove_file(&inc.join("lib/zlib_a.lib"));
//         t!(fs::hard_link(inc.join("lib/zlib.lib"), inc.join("lib/zlib_a.lib")));
//     }
//     run(&mut cmd);
//
//     let name = format!("libcurl-vc-{}-release-static-zlib-static-\
//                         ipv6-sspi-winssl", machine);
//     let libs = src.join("curl/builds").join(name);
//
//     t!(fs::copy(libs.join("lib/libcurl_a.lib"), dst.join("lib/curl.lib")));
//     for f in t!(fs::read_dir(libs.join("include/curl"))) {
//         let path = t!(f).path();
//         let dst = dst.join("include/curl").join(path.file_name().unwrap());
//         t!(fs::copy(path, dst));
//     }
//     t!(fs::remove_dir_all(src.join("curl/builds")));
//     println!("cargo:rustc-link-lib=wldap32");
//     println!("cargo:rustc-link-lib=advapi32");
//     println!("cargo:rustc-link-lib=normaliz");
// }
//
