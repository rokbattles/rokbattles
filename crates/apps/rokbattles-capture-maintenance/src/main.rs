fn main() {
    #[cfg(windows)]
    let result = rokbattles_capture_maintenance::windows::run();
    #[cfg(not(windows))]
    let result: std::io::Result<()> = Err(std::io::Error::new(
        std::io::ErrorKind::Unsupported,
        "capture maintenance requires native Windows",
    ));
    if let Err(error) = result {
        // No paths, account identities, IPC content or native response bodies.
        eprintln!(
            "ROK Battles capture maintenance failed ({:?}); repair may be required",
            error.kind()
        );
        std::process::exit(if error.kind() == std::io::ErrorKind::WouldBlock { 3010 } else { 1 });
    }
}
