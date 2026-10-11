#[cfg(target_os = "linux")]
mod linux;
#[cfg(target_os = "windows")]
mod windows;

#[cfg(any(test, target_os = "windows", target_os = "linux"))]
use std::collections::VecDeque;
#[cfg(any(test, target_os = "windows", target_os = "macos", target_os = "linux"))]
use std::{
    collections::BTreeSet,
    io,
    time::{Duration, Instant},
};
use std::{
    fs,
    path::{Path, PathBuf},
};

#[derive(Debug, Default)]
pub(crate) struct Discovery {
    pub(crate) dirs: Vec<PathBuf>,
    pub(crate) incomplete: bool,
}

pub(crate) fn discover_mailcache_dirs() -> Discovery {
    #[cfg(target_os = "windows")]
    {
        // Let the bounded search handle unavailable drives instead of probing them on the UI thread.
        let roots = (b'A'..=b'Z')
            .map(|letter| PathBuf::from(format!("{}:\\", char::from(letter))))
            .collect::<Vec<_>>();

        let mut search = windows_search();
        collect_windows_from_roots(&roots, &mut search, windows::collect_registered_installations);
        let mut result = Discovery::default();
        search.append_to(&mut result);
        dedupe_results(&mut result);

        return result;
    }

    #[cfg(target_os = "macos")]
    {
        let mut bases = Vec::new();

        if let Some(user_home) = std::env::var_os("HOME") {
            bases.push(PathBuf::from(user_home).join("Library").join("Containers"));
        }

        bases.push(PathBuf::from("/Library/Containers"));

        discover_macos_in_bases(&bases)
    }

    #[cfg(target_os = "linux")]
    {
        linux::discover()
    }

    #[cfg(not(any(target_os = "windows", target_os = "macos", target_os = "linux")))]
    Discovery::default()
}

#[cfg(any(test, target_os = "windows"))]
pub(crate) fn normalize_windows_path_for_display(path: &str) -> String {
    let normalized = path.trim().replace('/', "\\");

    if let Some(rest) = normalized.strip_prefix("\\\\?\\UNC\\") {
        return format!("\\\\{rest}");
    }

    if let Some(rest) = normalized.strip_prefix("\\\\?\\") {
        return rest.to_string();
    }

    normalized
}

pub(crate) fn path_identity_key(path: &Path) -> String {
    let normalized = fs::canonicalize(path).unwrap_or_else(|_| path.to_path_buf());
    let text = normalized.to_string_lossy().to_string();

    #[cfg(target_os = "windows")]
    {
        return normalize_windows_path_for_display(&text).to_ascii_lowercase();
    }

    #[cfg(not(target_os = "windows"))]
    text
}

#[cfg(any(test, target_os = "windows", target_os = "macos", target_os = "linux"))]
struct Search {
    deadline: Instant,
    remaining_operations: usize,
    checked: BTreeSet<PathBuf>,
    found: BTreeSet<PathBuf>,
    incomplete: bool,
}

#[cfg(any(test, target_os = "windows", target_os = "macos", target_os = "linux"))]
impl Search {
    fn new() -> Self {
        Self {
            deadline: Instant::now() + Duration::from_secs(2),
            remaining_operations: 3500,
            checked: BTreeSet::new(),
            found: BTreeSet::new(),
            incomplete: false,
        }
    }

    fn within_deadline(&mut self) -> bool {
        if Instant::now() >= self.deadline {
            self.incomplete = true;
            return false;
        }

        true
    }

    fn read_dir(&mut self, path: &Path) -> Option<fs::ReadDir> {
        if !self.within_deadline() || self.remaining_operations == 0 {
            self.incomplete = true;
            return None;
        }

        self.remaining_operations -= 1;

        match fs::read_dir(path) {
            Ok(entries) => Some(entries),
            Err(error) => {
                // Missing optional layouts and paths that disappeared are not unreadable installs.
                if !matches!(error.kind(), io::ErrorKind::NotFound | io::ErrorKind::NotADirectory) {
                    self.incomplete = true;
                }

                None
            }
        }
    }

    fn entries(&mut self, path: &Path, limit: usize) -> Vec<fs::DirEntry> {
        let Some(mut entries) = self.read_dir(path) else {
            return Vec::new();
        };
        let mut collected = Vec::new();

        // Errors consume the budget too; flatten().take(limit) can hide an unbounded error stream.
        for index in 0..=limit {
            if !self.within_deadline() {
                break;
            }

            let Some(entry) = entries.next() else {
                break;
            };

            if index == limit {
                self.incomplete = true;
                break;
            }

            match entry {
                Ok(entry) => collected.push(entry),
                Err(_) => self.incomplete = true,
            }
        }

        collected
    }

    fn child_dirs(&mut self, path: &Path, limit: usize) -> Vec<PathBuf> {
        let mut dirs = Vec::new();

        for entry in self.entries(path, limit) {
            if !self.within_deadline() {
                break;
            }

            match entry.file_type() {
                Ok(kind) if kind.is_dir() && !kind.is_symlink() => dirs.push(entry.path()),
                Ok(_) => {}
                Err(_) => self.incomplete = true,
            }
        }

        dirs
    }

    fn check_mailcache(&mut self, path: PathBuf) {
        if !self.checked.insert(path.clone()) {
            return;
        }

        let Some(mut entries) = self.read_dir(&path) else {
            return;
        };

        // Stop as soon as validity is established, even in a cache with many reports.
        for index in 0..=4096 {
            if !self.within_deadline() {
                return;
            }

            let Some(entry) = entries.next() else {
                return;
            };

            if index == 4096 {
                self.incomplete = true;
                return;
            }

            let Ok(entry) = entry else {
                self.incomplete = true;
                continue;
            };

            if !entry.file_name().to_str().is_some_and(is_persistent_mail_filename) {
                continue;
            }

            match entry.file_type() {
                Ok(kind) if kind.is_file() => {
                    self.found.insert(path);
                    return;
                }
                Ok(_) => {}
                Err(_) => self.incomplete = true,
            }
        }
    }

    fn append_to(self, result: &mut Discovery) {
        result.dirs.extend(self.found);
        result.incomplete |= self.incomplete;
    }
}

#[cfg(any(test, target_os = "windows", target_os = "macos", target_os = "linux"))]
fn dedupe_results(result: &mut Discovery) {
    let mut seen = BTreeSet::new();
    result.dirs.retain(|path| seen.insert(path_identity_key(path)));
    result.dirs.sort();
}

#[cfg(any(test, target_os = "windows"))]
fn windows_search() -> Search {
    let mut search = Search::new();
    search.deadline = Instant::now() + Duration::from_secs(8);
    search.remaining_operations = 14_000;
    search
}

#[cfg(test)]
fn discover_windows_from_roots(roots: &[PathBuf]) -> Discovery {
    let mut search = windows_search();
    collect_windows_from_roots(roots, &mut search, |_| {});
    let mut result = Discovery::default();
    search.append_to(&mut result);
    dedupe_results(&mut result);
    result
}

#[cfg(any(test, target_os = "windows"))]
fn collect_windows_from_roots(
    roots: &[PathBuf],
    search: &mut Search,
    collect_installation_hints: impl FnOnce(&mut Search),
) {
    // Establish direct standard installs across all drives before expanding any
    // directory tree. A large earlier drive must not hide a cheap later match.
    for root in roots {
        if !search.within_deadline() || search.remaining_operations == 0 {
            search.incomplete = true;
            return;
        }

        for base in ["Program Files (x86)", "Program Files"] {
            check_install_location(&root.join(base), search);
        }
    }

    for root in roots {
        if !search.within_deadline() || search.remaining_operations == 0 {
            search.incomplete = true;
            return;
        }

        for base in ["Program Files (x86)", "Program Files"] {
            collect_windows_candidates_from_base(&root.join(base), search);
        }
    }

    // Unrelated installer metadata must not exhaust the budget before standard probes.
    collect_installation_hints(search);

    for root in roots {
        if !search.within_deadline() || search.remaining_operations == 0 {
            search.incomplete = true;
            return;
        }

        collect_windows_candidates_fallback(root, search);
    }
}

#[cfg(any(test, target_os = "windows", target_os = "linux"))]
fn check_install_location(location: &Path, search: &mut Search) {
    search.check_mailcache(location.join("save").join("mailcache"));
    search.check_mailcache(location.join("Rise of Kingdoms Game").join("save").join("mailcache"));
}

#[cfg(any(test, target_os = "windows", target_os = "linux"))]
fn collect_windows_candidates_from_base(base: &Path, search: &mut Search) {
    check_install_location(base, search);

    for child in search.child_dirs(base, 1024) {
        check_install_location(&child, search);

        // Preserve the extra parent directory used by regional and nested installations.
        for nested in search.child_dirs(&child, 128) {
            check_install_location(&nested, search);
        }
    }
}

#[cfg(any(test, target_os = "windows", target_os = "linux"))]
fn collect_windows_candidates_fallback(root: &Path, search: &mut Search) {
    let mut queue = VecDeque::from([root.to_path_buf()]);
    let mut scheduled = 1;

    while let Some(dir) = queue.pop_front() {
        if !search.within_deadline() || search.remaining_operations == 0 {
            search.incomplete = true;
            break;
        }

        for child in search.child_dirs(&dir, 256) {
            let Some(name) = child.file_name().and_then(|name| name.to_str()) else {
                continue;
            };

            if name.eq_ignore_ascii_case("Rise of Kingdoms Game") {
                let mailcache = child.join("save").join("mailcache");

                if path_matches_windows_mailcache_suffix(&mailcache) {
                    search.check_mailcache(mailcache);
                }

                continue;
            }

            if should_skip_windows_dir(name) {
                continue;
            }

            // Bound the queued frontier as well as visited directories.
            if scheduled >= 3500 {
                search.incomplete = true;
                continue;
            }

            queue.push_back(child);
            scheduled += 1;
        }
    }
}

#[cfg(any(test, target_os = "windows", target_os = "linux"))]
fn should_skip_windows_dir(name: &str) -> bool {
    let lower = name.to_ascii_lowercase();
    matches!(
        lower.as_str(),
        "$recycle.bin"
            | "system volume information"
            | "windows"
            | "programdata"
            | "recovery"
            | "msocache"
            | "perflogs"
            | "target"
            | "node_modules"
            | ".git"
    )
}

#[cfg(any(test, target_os = "windows", target_os = "linux"))]
fn path_matches_windows_mailcache_suffix(path: &Path) -> bool {
    let normalized = path.to_string_lossy().replace('/', "\\").to_ascii_lowercase();
    normalized.ends_with("\\rise of kingdoms game\\save\\mailcache")
}

#[cfg(any(test, target_os = "macos"))]
fn discover_macos_in_bases(bases: &[PathBuf]) -> Discovery {
    let mut result = Discovery::default();

    for base in bases {
        let mut search = Search::new();

        for container in search.child_dirs(base, 2048) {
            let candidate = container.join("Data").join("Documents").join("mailcache");

            if path_matches_macos_container_pattern(&candidate) {
                search.check_mailcache(candidate);
            }
        }

        search.append_to(&mut result);
    }

    dedupe_results(&mut result);

    result
}

#[cfg(any(test, target_os = "macos"))]
fn path_matches_macos_container_pattern(path: &Path) -> bool {
    let normalized = path.to_string_lossy().replace('\\', "/");
    normalized.ends_with("/Data/Documents/mailcache") && normalized.contains("/Library/Containers/")
}

#[cfg(any(test, target_os = "windows", target_os = "macos", target_os = "linux"))]
fn is_persistent_mail_filename(name: &str) -> bool {
    let Some(rest) = name.strip_prefix("Persistent.Mail.") else {
        return false;
    };

    !rest.is_empty() && rest.bytes().all(|byte| byte.is_ascii_digit())
}

#[cfg(test)]
mod tests {
    use std::{fs, path::PathBuf};

    use tempfile::tempdir;

    use super::{
        discover_macos_in_bases, discover_windows_from_roots, is_persistent_mail_filename,
        normalize_windows_path_for_display, path_matches_windows_mailcache_suffix,
    };

    #[test]
    fn persistent_mail_filename_requires_numeric_suffix() {
        assert!(is_persistent_mail_filename("Persistent.Mail.123"));
        assert!(is_persistent_mail_filename("Persistent.Mail.001"));
        assert!(!is_persistent_mail_filename("Persistent.Mail."));
        assert!(!is_persistent_mail_filename("Persistent.Mail.123a"));
        assert!(!is_persistent_mail_filename("Persistent.mail.123"));
    }

    #[test]
    fn normalize_windows_path_removes_verbatim_prefix() {
        assert_eq!(
            normalize_windows_path_for_display(r"\\?\C:\Games\ROK\mailcache"),
            r"C:\Games\ROK\mailcache"
        );
        assert_eq!(
            normalize_windows_path_for_display(r"\\?\UNC\server\share\mailcache"),
            r"\\server\share\mailcache"
        );
    }

    #[test]
    fn windows_suffix_match_uses_stable_game_suffix() {
        assert!(path_matches_windows_mailcache_suffix(&PathBuf::from(
            r"C:\Anything\Rise of Kingdoms Game\save\mailcache"
        )));
        assert!(path_matches_windows_mailcache_suffix(&PathBuf::from(
            r"D:\Rise of Kingdoms KR\Rise of Kingdoms Game\save\mailcache"
        )));
        assert!(!path_matches_windows_mailcache_suffix(&PathBuf::from(
            r"D:\Rise of Kingdoms KR\mailcache"
        )));
    }

    #[test]
    fn windows_discovery_finds_parent_named_variants() {
        let temp = tempdir().expect("tempdir");
        let mailcache = temp
            .path()
            .join("Program Files (x86)")
            .join("Rise of Kingdoms KR")
            .join("Rise of Kingdoms Game")
            .join("save")
            .join("mailcache");
        fs::create_dir_all(&mailcache).expect("create dirs");
        fs::write(mailcache.join("Persistent.Mail.123"), b"test").expect("write file");

        let found = discover_windows_from_roots(&[temp.path().to_path_buf()]);
        assert!(found.dirs.contains(&mailcache));
    }

    #[test]
    fn windows_discovery_finds_nested_variant_when_standard_install_exists() {
        let temp = tempdir().expect("tempdir");
        let standard_mailcache = temp
            .path()
            .join("Program Files (x86)")
            .join("Rise of Kingdoms Game")
            .join("save")
            .join("mailcache");
        let nested_kr_mailcache = temp
            .path()
            .join("Program Files (x86)")
            .join("Rise of Kingdoms KR")
            .join("Custom Folder")
            .join("Rise of Kingdoms Game")
            .join("save")
            .join("mailcache");
        fs::create_dir_all(&standard_mailcache).expect("create standard dirs");
        fs::create_dir_all(&nested_kr_mailcache).expect("create nested kr dirs");
        fs::write(standard_mailcache.join("Persistent.Mail.1"), b"test")
            .expect("write standard file");
        fs::write(nested_kr_mailcache.join("Persistent.Mail.2"), b"test")
            .expect("write nested kr file");

        let found = discover_windows_from_roots(&[temp.path().to_path_buf()]);
        assert!(found.dirs.contains(&standard_mailcache));
        assert!(found.dirs.contains(&nested_kr_mailcache));
    }

    #[test]
    fn windows_fallback_finds_deeply_nested_variant() {
        let temp = tempdir().expect("tempdir");
        let deep_mailcache = temp
            .path()
            .join("Program Files")
            .join("Layer1")
            .join("Layer2")
            .join("Layer3")
            .join("Layer4")
            .join("Layer5")
            .join("Rise of Kingdoms Game")
            .join("save")
            .join("mailcache");
        fs::create_dir_all(&deep_mailcache).expect("create deep dirs");
        fs::write(deep_mailcache.join("Persistent.Mail.3"), b"test").expect("write deep file");

        let found = discover_windows_from_roots(&[temp.path().to_path_buf()]);
        assert!(found.dirs.contains(&deep_mailcache));
    }

    #[test]
    fn macos_discovery_uses_container_pattern_and_validation() {
        let temp = tempdir().expect("tempdir");
        let containers = temp.path().join("Library").join("Containers");
        let mailcache = containers
            .join("com.lilithgame.rok.kr")
            .join("Data")
            .join("Documents")
            .join("mailcache");
        fs::create_dir_all(&mailcache).expect("create dirs");
        fs::write(mailcache.join("Persistent.Mail.42"), b"test").expect("write file");

        let found = discover_macos_in_bases(&[containers]);
        assert_eq!(found.dirs, vec![mailcache]);
    }

    #[test]
    fn macos_discovery_rejects_non_mailcache_content() {
        let temp = tempdir().expect("tempdir");
        let containers = temp.path().join("Library").join("Containers");
        let mailcache =
            containers.join("com.lilithgame.rok").join("Data").join("Documents").join("mailcache");
        fs::create_dir_all(&mailcache).expect("create dirs");
        fs::write(mailcache.join("README.txt"), b"test").expect("write file");

        let found = discover_macos_in_bases(&[containers]);
        assert!(found.dirs.is_empty());
    }

    #[test]
    fn mixed_standard_and_custom_installations_on_multiple_roots_are_all_returned() {
        let temp = tempdir().expect("roots");
        let roots = [temp.path().join("C"), temp.path().join("D")];
        let expected = [
            roots[0].join("Program Files/Rise of Kingdoms Game/save/mailcache"),
            roots[0].join("Games/Rise of Kingdoms Game/save/mailcache"),
            roots[1].join("Custom/Rise of Kingdoms Game/save/mailcache"),
        ];
        for cache in &expected {
            fs::create_dir_all(cache).expect("cache directory");
            fs::write(cache.join("Persistent.Mail.1"), b"fixture").expect("report");
        }

        let result = discover_windows_from_roots(&roots);
        assert!(!result.incomplete);
        assert_eq!(result.dirs.len(), expected.len());
        for cache in expected {
            assert!(result.dirs.contains(&cache));
        }
    }

    #[test]
    fn exhausted_budgets_preserve_found_directories_and_report_partial_search() {
        let temp = tempdir().expect("cache");
        fs::write(temp.path().join("Persistent.Mail.1"), b"fixture").expect("report");
        let mut search = super::Search::new();
        search.remaining_operations = 1;
        search.check_mailcache(temp.path().to_path_buf());
        search.child_dirs(&temp.path().join("another location"), 256);
        let mut result = super::Discovery::default();
        search.append_to(&mut result);

        assert_eq!(result.dirs, vec![temp.path().to_path_buf()]);
        assert!(result.incomplete);
    }

    #[test]
    fn entry_and_time_limits_are_observable_without_discarding_results() {
        let temp = tempdir().expect("entries");
        for name in ["one", "two", "three"] {
            fs::create_dir(temp.path().join(name)).expect("entry");
        }
        let mut search = super::Search::new();
        assert_eq!(search.child_dirs(temp.path(), 2).len(), 2);
        assert!(search.incomplete);

        let mut search = super::Search::new();
        search.deadline = std::time::Instant::now();
        assert!(search.child_dirs(temp.path(), 10).is_empty());
        assert!(search.incomplete);
    }

    #[cfg(unix)]
    #[test]
    fn permission_failure_is_partial_and_does_not_hide_another_root() {
        use std::os::unix::fs::PermissionsExt;
        let temp = tempdir().expect("roots");
        let denied = temp.path().join("denied");
        let other = temp.path().join("other");
        fs::create_dir(&denied).expect("denied root");
        let cache = other.join("Games/Rise of Kingdoms Game/save/mailcache");
        fs::create_dir_all(&cache).expect("cache");
        fs::write(cache.join("Persistent.Mail.1"), b"fixture").expect("report");
        fs::set_permissions(&denied, fs::Permissions::from_mode(0o0)).expect("restrict access");
        let access_denied = fs::read_dir(&denied).is_err();
        let result = discover_windows_from_roots(&[denied.clone(), other]);
        fs::set_permissions(&denied, fs::Permissions::from_mode(0o700)).expect("restore access");

        assert_eq!(result.dirs, vec![cache]);
        if access_denied {
            assert!(result.incomplete);
        }
    }

    #[test]
    fn multiple_roots_cannot_reset_the_shared_windows_budget() {
        let temp = tempdir().expect("roots");
        let cache = temp.path().join("second/Program Files/Rise of Kingdoms Game/save/mailcache");
        fs::create_dir_all(&cache).expect("cache");
        fs::write(cache.join("Persistent.Mail.1"), b"fixture").expect("report");
        let roots = [temp.path().join("first"), temp.path().join("second")];
        let mut search = super::windows_search();
        search.remaining_operations = 1;
        super::collect_windows_from_roots(&roots, &mut search, |_| {});

        assert!(search.found.is_empty());
        assert!(search.incomplete);
        assert_eq!(search.remaining_operations, 0);

        let mut search = super::windows_search();
        search.deadline = std::time::Instant::now();
        super::collect_windows_from_roots(&roots, &mut search, |_| {});
        assert!(search.found.is_empty());
        assert!(search.incomplete);
    }

    #[test]
    fn exhausting_installation_metadata_cannot_hide_standard_caches() {
        let temp = tempdir().expect("root");
        let cache = temp.path().join("Program Files/Rise of Kingdoms Game/save/mailcache");
        fs::create_dir_all(&cache).expect("cache");
        fs::write(cache.join("Persistent.Mail.1"), b"fixture").expect("report");
        let mut search = super::windows_search();

        super::collect_windows_from_roots(&[temp.path().to_path_buf()], &mut search, |search| {
            search.remaining_operations = 0;
            search.incomplete = true;
        });

        assert!(search.found.contains(&cache));
        assert!(search.incomplete);
    }

    #[test]
    fn early_drive_expansion_cannot_hide_a_later_direct_standard_installation() {
        let temp = tempdir().expect("roots");
        let early = temp.path().join("C");
        let later = temp.path().join("Z");
        fs::create_dir_all(early.join("Program Files/Unrelated Application/Nested"))
            .expect("early expansion");
        let cache = later.join("Program Files/Rise of Kingdoms Game/save/mailcache");
        fs::create_dir_all(&cache).expect("cache");
        fs::write(cache.join("Persistent.Mail.1"), b"fixture").expect("report");
        let mut search = super::windows_search();
        search.remaining_operations = 12;

        super::collect_windows_from_roots(&[early, later], &mut search, |_| {});

        assert!(search.found.contains(&cache));
        assert!(search.incomplete);
        assert_eq!(search.remaining_operations, 0);
    }
}
