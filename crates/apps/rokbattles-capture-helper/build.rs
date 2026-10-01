fn main() {
    println!("cargo:rerun-if-changed=src/unix/macos_shim.c");
    println!("cargo:rerun-if-changed=src/unix/macos_auth.c");
    if std::env::var("CARGO_CFG_TARGET_OS").as_deref() == Ok("macos") {
        cc::Build::new()
            .file("src/unix/macos_shim.c")
            .file("src/unix/macos_auth.c")
            .warnings(true)
            .warnings_into_errors(true)
            .compile("rok_capture_proc");
        println!("cargo:rustc-link-lib=framework=Security");
        println!("cargo:rustc-link-lib=framework=CoreFoundation");
    }
}
