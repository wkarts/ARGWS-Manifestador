use std::{env, fs, path::PathBuf};
use sha2::{Digest, Sha256};
fn main() {
    let archive = "vendor/libxml2-2.15.4.tar.gz";
    println!("cargo:rerun-if-changed={archive}");
    let bytes = fs::read(archive).expect("bundled libxml2 source");
    assert_eq!(
        format!("{:x}", Sha256::digest(&bytes)),
        "14f2d86b5d34f23e8651d07d070ab3c651e8100e2dc8a103b50f18fd2f408499",
        "libxml2 source checksum mismatch"
    );
    let out = PathBuf::from(env::var_os("OUT_DIR").unwrap());
    let source = out.join("libxml2");
    if !source.join("CMakeLists.txt").is_file() {
        tar::Archive::new(flate2::read::GzDecoder::new(bytes.as_slice()))
            .unpack(&out)
            .expect("extract pinned libxml2 source");
    }
    let target = env::var("TARGET").unwrap();
    let mut build = cmake::Config::new(source);
    build
        .profile("Release")
        .define("BUILD_SHARED_LIBS", "OFF")
        .define("CMAKE_POSITION_INDEPENDENT_CODE", "ON")
        .define("CMAKE_C_VISIBILITY_PRESET", "hidden")
        .define("CMAKE_INSTALL_LIBDIR", "lib");
    for feature in [
        "PROGRAMS", "TESTS", "PYTHON", "DOCS", "ICONV", "ICU", "ZLIB", "LZMA", "MODULES", "HTTP",
        "CATALOG",
    ] {
        build.define(format!("LIBXML2_WITH_{feature}"), "OFF");
    }
    build.define("LIBXML2_WITH_SCHEMAS", "ON");
    if target.contains("android") {
        let ndk = env::var_os("ANDROID_NDK_HOME")
            .or_else(|| env::var_os("NDK_HOME"))
            .expect("Android NDK supplied by Tauri build");
        build.define(
            "CMAKE_TOOLCHAIN_FILE",
            PathBuf::from(ndk).join("build/cmake/android.toolchain.cmake"),
        );
        let abi = if target.starts_with("aarch64") {
            "arm64-v8a"
        } else if target.starts_with("armv7") {
            "armeabi-v7a"
        } else if target.starts_with("x86_64") {
            "x86_64"
        } else {
            "x86"
        };
        build
            .define("ANDROID_ABI", abi)
            .define("ANDROID_PLATFORM", "android-24");
    }
    let dst = build.build();
    println!("cargo:rustc-link-search=native={}/lib", dst.display());
    println!("cargo:rustc-link-lib=static=xml2");
    if target.contains("windows") {
        println!("cargo:rustc-link-lib=bcrypt");
    } else {
        println!("cargo:rustc-link-lib=m");
    }
}
