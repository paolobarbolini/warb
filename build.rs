fn main() {
    let mut build = cxx_build::bridge("src/lib.rs");
    build.file("cpp/warb_shim.cc").include("cpp").std("c++17");
    if std::env::var_os("CARGO_FEATURE_TRACE").is_some() {
        build.define("WARB_TRACE", None);
    }
    build.compile("warb_shim");

    println!("cargo:rerun-if-changed=src/lib.rs");
    println!("cargo:rerun-if-changed=cpp/warb_shim.cc");
    println!("cargo:rerun-if-changed=cpp/warb_shim.h");
    println!("cargo:rerun-if-changed=cpp/warb_abi.h");
    println!("cargo:rerun-if-changed=cpp/warb_dispatch.h");
    println!("cargo:rerun-if-env-changed=BRAW_RUNTIME_DIR");
    println!("cargo:rustc-link-lib=dylib=dl");
}
