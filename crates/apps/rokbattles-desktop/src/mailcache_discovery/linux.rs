#[path = "steam.rs"]
mod steam;

use std::{
    collections::BTreeSet,
    fs,
    io::{self, Read},
    path::{Path, PathBuf},
    time::{Duration, Instant},
};

use serde::Deserialize;

use super::{
    Discovery, Search, check_install_location, collect_windows_candidates_fallback,
    collect_windows_candidates_from_base, dedupe_results, path_identity_key,
};

struct Locations {
    home: PathBuf,
    data: PathBuf,
    config: PathBuf,
    wine_prefix: Option<PathBuf>,
}

pub(super) fn discover() -> Discovery {
    let Some(home) = std::env::var_os("HOME").map(PathBuf::from).filter(|path| path.is_absolute())
    else {
        return Discovery { incomplete: true, ..Discovery::default() };
    };
    let locations = Locations {
        data: std::env::var_os("XDG_DATA_HOME")
            .map(PathBuf::from)
            .filter(|path| path.is_absolute())
            .unwrap_or_else(|| home.join(".local/share")),
        config: std::env::var_os("XDG_CONFIG_HOME")
            .map(PathBuf::from)
            .filter(|path| path.is_absolute())
            .unwrap_or_else(|| home.join(".config")),
        wine_prefix: std::env::var_os("WINEPREFIX")
            .map(PathBuf::from)
            .filter(|path| path.is_absolute()),
        home,
    };

    discover_in(&locations)
}

#[derive(Default)]
struct Hints {
    prefixes: BTreeSet<PathBuf>,
    installs: BTreeSet<PathBuf>,
}

impl Hints {
    fn prefix(&mut self, path: PathBuf, search: &mut Search) {
        Self::insert(&mut self.prefixes, path, search);
    }

    fn install(&mut self, path: PathBuf, search: &mut Search) {
        Self::insert(&mut self.installs, path, search);
    }

    fn insert(paths: &mut BTreeSet<PathBuf>, path: PathBuf, search: &mut Search) {
        if !path.is_absolute() {
            return;
        }
        if paths.len() >= 128 {
            search.incomplete = true;
            return;
        }

        paths.insert(path);
    }
}

fn discover_in(locations: &Locations) -> Discovery {
    let mut metadata = Search::new();
    let mut hints = Hints::default();
    hints.prefix(locations.home.join(".wine"), &mut metadata);

    if let Some(prefix) = &locations.wine_prefix {
        hints.prefix(prefix.clone(), &mut metadata);
    }

    collect_steam(locations, &mut metadata, &mut hints);
    collect_lutris(locations, &mut metadata, &mut hints);
    collect_bottles(locations, &mut metadata, &mut hints);

    // Common manually created Wine prefixes and Lutris installations.
    for base in [locations.home.join("Games"), locations.data.join("lutris/prefixes")] {
        for prefix in metadata.child_dirs(&base, 128) {
            hints.prefix(prefix, &mut metadata);
        }
    }

    let mut result = Discovery::default();
    metadata.append_to(&mut result);
    let mut standard = Search::new();

    for install in hints.installs {
        check_install_location(&install, &mut standard);
    }

    let mut roots = Vec::new();
    let mut seen = BTreeSet::new();

    for prefix in hints.prefixes {
        if !standard.within_deadline() {
            break;
        }

        let root = prefix.join("drive_c");

        // Never follow Wine's dosdevices/z: mapping, or a drive_c redirected to the host root.
        match fs::symlink_metadata(&root) {
            Ok(info) if info.is_dir() && !info.file_type().is_symlink() => {}
            Ok(_) => continue,
            Err(error) => {
                if error.kind() != io::ErrorKind::NotFound {
                    standard.incomplete = true;
                }
                continue;
            }
        }

        if !seen.insert(path_identity_key(&root)) {
            continue;
        }

        for base in ["Program Files", "Program Files (x86)"] {
            collect_windows_candidates_from_base(&root.join(base), &mut standard);
        }

        roots.push(root);
    }

    standard.append_to(&mut result);

    // Each prefix gets a bounded fallback; a shared deadline bounds a large prefix collection.
    let deadline = Instant::now() + Duration::from_secs(6);
    for root in roots {
        if Instant::now() >= deadline {
            result.incomplete = true;
            break;
        }

        let mut search = Search::new();
        search.deadline = search.deadline.min(deadline);
        collect_windows_candidates_fallback(&root, &mut search);
        search.append_to(&mut result);
    }

    dedupe_results(&mut result);
    result
}

fn collect_steam(locations: &Locations, search: &mut Search, hints: &mut Hints) {
    let flatpak = locations.home.join(".var/app/com.valvesoftware.Steam");
    let roots = [
        locations.data.join("Steam"),
        locations.data.join("steam"),
        locations.home.join(".steam/steam"),
        locations.home.join(".steam/root"),
        locations.home.join(".steam/debian-installation"),
        locations.home.join("snap/steam/common/.local/share/Steam"),
        flatpak.join(".local/share/Steam"),
        flatpak.join("data/Steam"),
        flatpak.join("data/steam"),
    ];
    let mut libraries = BTreeSet::new();

    for root in roots {
        libraries.insert(root.clone());
        for path in
            [root.join("steamapps/libraryfolders.vdf"), root.join("config/libraryfolders.vdf")]
        {
            let Some(text) = read_metadata(&path, search) else {
                continue;
            };

            match steam::library_paths(&text) {
                Ok(paths) => {
                    for path in paths {
                        if libraries.len() >= 128 {
                            search.incomplete = true;
                            break;
                        }
                        libraries.insert(path);
                    }
                }
                Err(()) => search.incomplete = true,
            }
        }
    }

    let mut seen = BTreeSet::new();
    for library in libraries {
        if !search.within_deadline() {
            break;
        }
        if !seen.insert(path_identity_key(&library)) {
            continue;
        }

        for game in search.child_dirs(&library.join("steamapps/compatdata"), 1024) {
            hints.prefix(game.join("pfx"), search);
        }
    }
}

#[derive(Deserialize)]
struct LutrisConfig {
    game: Option<LutrisGame>,
}

#[derive(Deserialize)]
struct LutrisGame {
    prefix: Option<String>,
    exe: Option<String>,
}

fn collect_lutris(locations: &Locations, search: &mut Search, hints: &mut Hints) {
    let flatpak = locations.home.join(".var/app/net.lutris.Lutris");
    let bases = [
        locations.data.join("lutris/games"),
        locations.config.join("lutris/games"),
        flatpak.join("data/lutris/games"),
        flatpak.join("config/lutris/games"),
    ];

    for base in bases {
        for entry in search.entries(&base, 256) {
            let path = entry.path();
            if !matches!(path.extension().and_then(|value| value.to_str()), Some("yml" | "yaml")) {
                continue;
            }
            let Some(text) = read_metadata(&path, search) else {
                continue;
            };
            let game = match yaml_serde::from_str::<LutrisConfig>(&text) {
                Ok(config) => config.game,
                Err(_) => {
                    search.incomplete = true;
                    continue;
                }
            };
            let Some(game) = game else {
                continue;
            };
            let prefix =
                game.prefix.as_deref().and_then(|value| absolute_path(value, &locations.home));

            if let Some(exe) = game.exe.as_deref() {
                let executable = absolute_path(exe, &locations.home).or_else(|| {
                    let relative = exe.replace('\\', "/");
                    let relative = relative
                        .strip_prefix("C:/")
                        .or_else(|| relative.strip_prefix("c:/"))
                        .map(|path| PathBuf::from("drive_c").join(path))
                        .unwrap_or_else(|| PathBuf::from(relative));

                    prefix.as_ref().map(|prefix| prefix.join(relative))
                });
                if let Some(parent) = executable.as_deref().and_then(Path::parent) {
                    hints.install(parent.to_path_buf(), search);
                }
            }

            if let Some(prefix) = prefix {
                hints.prefix(prefix, search);
            }
        }
    }
}

#[derive(Deserialize)]
struct BottlePlaceholder {
    #[serde(rename = "Path")]
    path: String,
}

fn collect_bottles(locations: &Locations, search: &mut Search, hints: &mut Hints) {
    let bases = [
        locations.data.join("bottles/bottles"),
        locations.home.join(".var/app/com.usebottles.bottles/data/bottles/bottles"),
    ];

    for base in bases {
        for bottle in search.child_dirs(&base, 128) {
            hints.prefix(bottle.clone(), search);
            let Some(text) = read_metadata(&bottle.join("placeholder.yml"), search) else {
                continue;
            };

            match yaml_serde::from_str::<BottlePlaceholder>(&text) {
                Ok(placeholder) => {
                    if let Some(path) = absolute_path(&placeholder.path, &locations.home) {
                        hints.prefix(path, search);
                    }
                }
                Err(_) => search.incomplete = true,
            }
        }
    }
}

fn absolute_path(value: &str, home: &Path) -> Option<PathBuf> {
    for prefix in ["~/", "$HOME/", "${HOME}/"] {
        if let Some(relative) = value.strip_prefix(prefix) {
            return Some(home.join(relative));
        }
    }

    let path = PathBuf::from(value);
    path.is_absolute().then_some(path)
}

fn read_metadata(path: &Path, search: &mut Search) -> Option<String> {
    if !search.within_deadline() || search.remaining_operations == 0 {
        search.incomplete = true;
        return None;
    }
    search.remaining_operations -= 1;

    let read = || -> io::Result<String> {
        let mut options = fs::OpenOptions::new();
        options.read(true);

        // A replaced launcher file must not turn into a blocking FIFO or device read.
        #[cfg(unix)]
        {
            use std::os::unix::fs::OpenOptionsExt;
            options.custom_flags(libc::O_NONBLOCK | libc::O_NOFOLLOW);
        }

        let file = options.open(path)?;
        let metadata = file.metadata()?;
        if !metadata.is_file() || metadata.len() > 262_144 {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "Launcher metadata is not a small regular file",
            ));
        }

        let mut text = String::new();
        file.take(262_145).read_to_string(&mut text)?;
        if text.len() > 262_144 {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "Launcher metadata grew beyond the limit",
            ));
        }
        Ok(text)
    };

    match read() {
        Ok(text) => Some(text),
        Err(error) => {
            if !matches!(error.kind(), io::ErrorKind::NotFound | io::ErrorKind::NotADirectory) {
                search.incomplete = true;
            }
            None
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn locations(home: &Path) -> Locations {
        Locations {
            home: home.to_path_buf(),
            data: home.join(".local/share"),
            config: home.join(".config"),
            wine_prefix: None,
        }
    }

    fn mailcache(prefix: &Path) -> PathBuf {
        let cache = prefix.join("drive_c/Program Files/Rise of Kingdoms Game/save/mailcache");
        fs::create_dir_all(&cache).expect("cache directory");
        fs::write(cache.join("Persistent.Mail.42"), b"fixture").expect("report");
        cache
    }

    fn metadata(path: &Path, text: &str) {
        fs::create_dir_all(path.parent().expect("metadata parent")).expect("metadata directory");
        fs::write(path, text).expect("metadata file");
    }

    #[test]
    fn discovers_wine_external_proton_lutris_and_bottles_together() {
        let temp = tempfile::tempdir().expect("home");
        let locations = locations(temp.path());
        let library = temp.path().join("external Steam library");
        let lutris = temp.path().join("custom Lutris prefix");
        let bottles = temp.path().join("custom Bottle");
        let expected = [
            mailcache(&locations.home.join(".wine")),
            mailcache(&library.join("steamapps/compatdata/123/pfx")),
            mailcache(&lutris),
            mailcache(&bottles),
        ];
        metadata(
            &locations.data.join("Steam/steamapps/libraryfolders.vdf"),
            &format!("\"libraryfolders\" {{ \"1\" {{ \"path\" \"{}\" }} }}", library.display()),
        );
        metadata(
            &locations.data.join("lutris/games/rok.yml"),
            &format!("game:\n  prefix: '{}'\n", lutris.display()),
        );
        metadata(
            &locations.data.join("bottles/bottles/ROK/placeholder.yml"),
            &format!("Path: '{}'\n", bottles.display()),
        );

        let result = discover_in(&locations);
        assert!(!result.incomplete);
        assert_eq!(result.dirs.len(), expected.len());
        for cache in expected {
            assert!(result.dirs.contains(&cache));
        }
    }

    #[test]
    fn discovers_flatpak_layouts_and_explicit_wine_prefix() {
        let temp = tempfile::tempdir().expect("home");
        let mut locations = locations(temp.path());
        locations.wine_prefix = Some(temp.path().join("environment prefix"));
        let lutris = temp.path().join("flatpak Lutris prefix");
        let expected = [
            mailcache(locations.wine_prefix.as_deref().expect("prefix")),
            mailcache(&locations.home.join(
                ".var/app/com.valvesoftware.Steam/.local/share/Steam/steamapps/compatdata/456/pfx",
            )),
            mailcache(
                &locations.home.join(".var/app/com.usebottles.bottles/data/bottles/bottles/ROK"),
            ),
            mailcache(&lutris),
        ];
        metadata(
            &locations.home.join(".var/app/net.lutris.Lutris/config/lutris/games/regional.yml"),
            &format!("game:\n  prefix: '{}'\n", lutris.display()),
        );

        let result = discover_in(&locations);
        assert!(!result.incomplete);
        for cache in expected {
            assert!(result.dirs.contains(&cache));
        }
    }

    #[test]
    fn validates_localized_installation_from_executable_hint() {
        let temp = tempfile::tempdir().expect("home");
        let locations = locations(temp.path());
        let install = temp.path().join("external games/万国觉醒");
        let cache = install.join("save/mailcache");
        fs::create_dir_all(&cache).expect("cache");
        fs::write(cache.join("Persistent.Mail.2"), b"fixture").expect("report");
        metadata(
            &locations.config.join("lutris/games/client.yml"),
            &format!("game:\n  exe: '{}/game.exe'\n", install.display()),
        );

        assert_eq!(discover_in(&locations).dirs, vec![cache]);
    }

    #[test]
    fn malformed_or_oversized_metadata_preserves_valid_results_as_partial() {
        let temp = tempfile::tempdir().expect("home");
        let locations = locations(temp.path());
        let cache = mailcache(&locations.home.join(".wine"));
        metadata(
            &locations.data.join("Steam/steamapps/libraryfolders.vdf"),
            "libraryfolders { \"0\"",
        );
        metadata(&locations.data.join("lutris/games/bad.yml"), "game: [");
        metadata(&locations.data.join("lutris/games/large.yml"), &" ".repeat(262_145));

        let result = discover_in(&locations);
        assert!(result.incomplete);
        assert_eq!(result.dirs, vec![cache]);
    }

    #[cfg(unix)]
    #[test]
    fn does_not_follow_host_drive_mapping_or_prefix_symlink_cycles() {
        use std::os::unix::fs::symlink;
        let temp = tempfile::tempdir().expect("home");
        let locations = locations(temp.path());
        let cache = mailcache(&locations.home.join(".wine"));
        let host = temp.path().join("host");
        let hidden = mailcache(&host);
        fs::create_dir_all(locations.home.join(".wine/dosdevices")).expect("mapping directory");
        symlink(&host, locations.home.join(".wine/dosdevices/z:")).expect("host mapping");
        symlink(locations.home.join(".wine/drive_c"), locations.home.join(".wine/drive_c/loop"))
            .expect("loop");
        symlink(&host, locations.home.join(".wine/drive_c/external")).expect("external link");

        let result = discover_in(&locations);
        assert!(result.dirs.contains(&cache));
        assert!(!result.dirs.contains(&hidden));
    }

    #[cfg(unix)]
    #[test]
    fn metadata_fifo_and_symlink_are_rejected_without_reading_them() {
        use std::os::unix::fs::symlink;
        let temp = tempfile::tempdir().expect("metadata root");
        let pipe = temp.path().join("launcher.yml");
        let status = std::process::Command::new("mkfifo").arg(&pipe).status().expect("mkfifo");
        assert!(status.success());
        let alias = temp.path().join("alias.yml");
        symlink(&pipe, &alias).expect("metadata alias");

        let mut search = Search::new();

        assert!(read_metadata(&pipe, &mut search).is_none());
        assert!(read_metadata(&alias, &mut search).is_none());
        assert!(search.incomplete);
    }
}
