use std::{io, path::PathBuf};

use winreg::{
    RegKey,
    enums::{HKEY_CURRENT_USER, HKEY_LOCAL_MACHINE, KEY_READ, KEY_WOW64_32KEY, KEY_WOW64_64KEY},
};

use super::{Search, check_install_location};

pub(super) fn collect_registered_installations(search: &mut Search) {
    // Display names vary by region. Treat installer locations as hints and validate
    // the cache itself; never execute uninstall strings or launcher commands.
    for hive in [HKEY_CURRENT_USER, HKEY_LOCAL_MACHINE] {
        for view in [KEY_WOW64_32KEY, KEY_WOW64_64KEY] {
            if !search.within_deadline() {
                break;
            }

            let root = RegKey::predef(hive);
            let uninstall = match root.open_subkey_with_flags(
                r"Software\Microsoft\Windows\CurrentVersion\Uninstall",
                KEY_READ | view,
            ) {
                Ok(key) => key,
                Err(error) => {
                    if error.kind() != io::ErrorKind::NotFound {
                        search.incomplete = true;
                    }
                    continue;
                }
            };

            for (index, name) in uninstall.enum_keys().enumerate() {
                if index >= 2048 || !search.within_deadline() || search.remaining_operations == 0 {
                    search.incomplete = true;
                    break;
                }

                search.remaining_operations -= 1;

                let location = name
                    .and_then(|name| uninstall.open_subkey_with_flags(name, KEY_READ | view))
                    .and_then(|key| key.get_value::<String, _>("InstallLocation"));

                match location {
                    Ok(value) => {
                        let location = PathBuf::from(value.trim().trim_matches('"'));

                        if location.is_absolute() {
                            check_install_location(&location, search);
                        }
                    }
                    Err(error) if error.kind() == io::ErrorKind::NotFound => {}
                    Err(_) => search.incomplete = true,
                }
            }
        }
    }
}
