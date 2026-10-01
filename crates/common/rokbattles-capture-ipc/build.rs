fn main() {
    println!("cargo:rerun-if-changed=src/unix_trust_macos.c");
    if std::env::var("CARGO_CFG_TARGET_OS").as_deref() == Ok("macos") {
        cc::Build::new()
            .file("src/unix_trust_macos.c")
            .warnings(true)
            .warnings_into_errors(true)
            .compile("rok_capture_acl");
    }
}
