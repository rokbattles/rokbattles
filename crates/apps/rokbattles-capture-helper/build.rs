fn main() {
    println!("cargo:rerun-if-changed=src/unix/macos_shim.c");
    if std::env::var("CARGO_CFG_TARGET_OS").as_deref() == Ok("macos") {
        cc::Build::new()
            .file("src/unix/macos_shim.c")
            .warnings(true)
            .warnings_into_errors(true)
            .compile("rok_capture_proc");
    }
}
