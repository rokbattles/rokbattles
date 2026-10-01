fn main() {
    let arguments = std::env::args_os().skip(1).collect::<Vec<_>>();
    #[cfg(windows)]
    let result = if arguments == [std::ffi::OsString::from("--service")] {
        rokbattles_capture_helper::windows::dispatch()
    } else {
        Err(std::io::Error::other("installed service mode required"))
    };
    #[cfg(any(target_os = "linux", target_os = "macos"))]
    let result = match rokbattles_capture_helper::unix::service_uid(&arguments) {
        Some(uid) => rokbattles_capture_helper::unix::dispatch(uid),
        None => Err(std::io::Error::other("installed service mode required")),
    };
    #[cfg(not(any(windows, target_os = "linux", target_os = "macos")))]
    let result: std::io::Result<()> = Err(std::io::Error::other("unsupported capture platform"));
    if result.is_err() {
        eprintln!(
            "The capture helper must be provisioned and started by its operating-system service."
        );
        std::process::exit(1);
    }
}
