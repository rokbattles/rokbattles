fn main() {
    // No foreground capture, arbitrary path/filter flags, installer or elevation
    // entrypoint. The explicit maintenance installer registers this SCM-only mode.
    if std::env::args_os().skip(1).collect::<Vec<_>>() != [std::ffi::OsString::from("--service")] {
        eprintln!("This capture helper must be started by its installed operating-system service.");
        std::process::exit(2);
    }
    #[cfg(windows)]
    if rokbattles_capture_helper::windows::dispatch().is_err() {
        eprintln!("The capture helper service is unavailable.");
        std::process::exit(1);
    }
    #[cfg(not(windows))]
    {
        eprintln!("Capture service provisioning is not available for this platform.");
        std::process::exit(2);
    }
}
