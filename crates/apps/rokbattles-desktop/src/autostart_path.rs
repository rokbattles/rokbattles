//! Command representations for auto-launch 0.5, which writes paths verbatim.
//! These functions accept an executable path only, never a command or arguments.

#[cfg(any(target_os = "windows", test))]
pub(crate) fn windows_executable(path: &str) -> Result<String, String> {
    if path.is_empty() || path.contains(['\0', '"']) {
        return Err("Invalid Windows automatic startup executable path.".to_string());
    }
    // argv[0] is quoted as a whole; backslashes in an executable path are literal.
    // No cmd.exe or shell is involved in the Run entry.
    Ok(format!("\"{path}\""))
}

#[cfg(any(target_os = "linux", test))]
pub(crate) fn linux_path<'a>(
    current_exe: &'a std::path::Path,
    appimage: Option<&'a std::ffi::OsStr>,
) -> &'a std::path::Path {
    appimage.map(std::path::Path::new).unwrap_or(current_exe)
}

#[cfg(any(target_os = "linux", test))]
pub(crate) fn linux_executable(path: &str) -> Result<String, String> {
    if path.is_empty() || path.contains(['\0', '=']) {
        return Err("Invalid Linux automatic startup executable path.".to_string());
    }
    // Desktop Entry spec: string-value escaping is decoded before Exec quoting,
    // then field codes are expanded. This is not shell escaping.
    // https://specifications.freedesktop.org/desktop-entry/latest/exec-variables.html
    let mut command = String::from("\"");
    for character in path.chars() {
        match character {
            '\\' => command.push_str("\\\\\\\\"),
            '"' | '`' | '$' => {
                command.push_str("\\\\");
                command.push(character);
            }
            '\n' => command.push_str("\\n"),
            '\r' => command.push_str("\\r"),
            '\t' => command.push_str("\\t"),
            '%' => command.push_str("%%"),
            _ => command.push(character),
        }
    }
    command.push('"');
    Ok(command)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn windows_quotes_the_entire_executable_path() {
        for path in [
            r"C:\Users\First Last\AppData\Local\ROK Battles\rokbattles-desktop.exe",
            r"C:\Program Files\ROK Battles\rokbattles-desktop.exe",
            r"C:\ROKBattles\rokbattles-desktop.exe",
            r"C:\Users\雪 😀\ROK Battles.exe",
            r"\\server\shared apps\ROK Battles.exe",
            r"\\?\C:\Users\First Last\ROK Battles.exe",
        ] {
            assert_eq!(windows_executable(path), Ok(format!("\"{path}\"")));
        }
    }

    #[test]
    fn windows_rejects_commands_and_invalid_paths() {
        for path in ["", "bad\0path.exe", r#"C:\bad"path.exe"#, r#""C:\app.exe" --flag"#] {
            windows_executable(path).expect_err("invalid Windows path should be rejected");
        }
    }

    #[test]
    fn linux_quotes_spaces_and_preserves_unicode() {
        for path in [
            "/home/First Last/Applications/ROK Battles.AppImage",
            "/usr/bin/rokbattles-desktop",
            "/home/雪 😀/ROK Battles.AppImage",
            "/home/user/it's (a) #test;&|<>~*?.AppImage",
        ] {
            assert_eq!(linux_executable(path), Ok(format!("\"{path}\"")));
        }
    }

    #[test]
    fn linux_escapes_both_desktop_entry_layers_and_literal_field_codes() {
        assert_eq!(
            linux_executable("/tmp/a\\b\"c`d$e%f.AppImage"),
            Ok(r#""/tmp/a\\\\b\\"c\\`d\\$e%%f.AppImage""#.to_string()),
        );
        assert_eq!(
            linux_executable("/tmp/a\nb\rc\td.AppImage"),
            Ok(r#""/tmp/a\nb\rc\td.AppImage""#.to_string()),
        );
    }

    #[test]
    fn linux_rejects_invalid_desktop_entry_executable_paths() {
        for path in ["", "/tmp/bad\0path", "/tmp/a=b.AppImage"] {
            linux_executable(path).expect_err("invalid desktop entry path should be rejected");
        }
    }

    #[test]
    fn linux_prefers_the_original_appimage_path() {
        use std::path::Path;

        let mounted = Path::new("/tmp/.mount_ROK/usr/bin/rokbattles-desktop");
        let appimage = Path::new("/home/First Last/ROK Battles.AppImage");
        assert_eq!(linux_path(mounted, Some(appimage.as_os_str())), appimage);
        assert_eq!(linux_path(mounted, None), mounted);
    }

    #[cfg(target_os = "linux")]
    #[test]
    fn linux_backend_replaces_and_removes_legacy_entry() {
        use std::{env, fs, process::Command};

        const TEST_HOME: &str = "ROKBATTLES_AUTOSTART_TEST_HOME";
        // The child owns an isolated HOME before any threads start. Never alter
        // the developer's login items or mutate the test process environment.
        let Some(home) = env::var_os(TEST_HOME) else {
            let home = tempfile::tempdir().expect("isolated home");
            let status = Command::new(env::current_exe().expect("test executable"))
                .arg("linux_backend_replaces_and_removes_legacy_entry")
                .arg("--nocapture")
                .env("HOME", home.path())
                .env(TEST_HOME, home.path())
                .status()
                .expect("run isolated backend test");
            assert!(status.success());
            return;
        };
        let directory = std::path::PathBuf::from(home).join(".config/autostart");
        fs::create_dir_all(&directory).expect("create isolated autostart directory");
        let file = directory.join("ROK Battles Test.desktop");
        let path = "/home/First Last/ROK Battles.AppImage";
        let legacy = format!("[Desktop Entry]\nType=Application\nExec={path} \n");
        fs::write(&file, &legacy).expect("seed legacy entry");

        let command = linux_executable(path).expect("quote executable path");
        let launcher = auto_launch::AutoLaunch::new("ROK Battles Test", &command, &[] as &[&str]);
        launcher.enable().expect("replace legacy entry");
        let contents = fs::read_to_string(&file).expect("read replaced entry");
        assert!(contents.lines().any(|line| line == format!("Exec={command} ")));
        assert_eq!(fs::read_dir(&directory).expect("read isolated directory").count(), 1);
        assert!(launcher.is_enabled().expect("read startup state"));

        // Disabled startup only requires the same entry name; an unencodable
        // executable must not prevent removing a previously enabled entry.
        fs::write(&file, legacy).expect("restore legacy entry");
        let disabled = auto_launch::AutoLaunch::new("ROK Battles Test", "", &[] as &[&str]);
        disabled.disable().expect("disable legacy entry");
        assert!(!file.exists());
        assert!(!disabled.is_enabled().expect("read disabled startup state"));
        disabled.disable().expect("disable legacy entry");
    }
}
