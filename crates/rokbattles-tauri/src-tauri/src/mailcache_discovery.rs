#![cfg_attr(
    not(any(test, target_os = "windows", target_os = "macos")),
    allow(dead_code)
)]

use std::{
    collections::BTreeSet,
    fs,
    path::{Path, PathBuf},
};
#[cfg(any(test, target_os = "windows"))]
use std::{
    collections::VecDeque,
    time::{Duration, Instant},
};

#[cfg(any(test, target_os = "windows"))]
const WINDOWS_MAILCACHE_SUFFIX_LOWER: &str = "\\rise of kingdoms game\\save\\mailcache";

#[cfg(any(test, target_os = "windows"))]
const WINDOWS_BASE_DIRS: &[&str] = &["Program Files (x86)", "Program Files"];
#[cfg(any(test, target_os = "windows"))]
const MAX_WINDOWS_BASE_CHILDREN: usize = 1024;
#[cfg(any(test, target_os = "windows"))]
const MAX_WINDOWS_BASE_GRANDCHILDREN: usize = 128;
#[cfg(any(test, target_os = "windows"))]
const MAX_WINDOWS_FALLBACK_DIRS: usize = 3500;
#[cfg(any(test, target_os = "windows"))]
const MAX_WINDOWS_FALLBACK_CHILDREN_PER_DIR: usize = 256;
#[cfg(any(test, target_os = "windows"))]
const MAX_WINDOWS_FALLBACK_DURATION: Duration = Duration::from_secs(2);

#[cfg(any(test, target_os = "macos"))]
const MAX_MACOS_CONTAINERS: usize = 2048;
const MAX_MAILCACHE_FILES_TO_CHECK: usize = 4096;

#[cfg(any(test, target_os = "windows"))]
pub(crate) fn normalize_windows_path_for_display(path: &str) -> String {
    let normalized = path.trim().replace('/', "\\");
    if let Some(rest) = normalized.strip_prefix("\\\\?\\UNC\\") {
        return format!("\\\\{}", rest);
    }
    if let Some(rest) = normalized.strip_prefix("\\\\?\\") {
        return rest.to_string();
    }
    normalized
}

pub(crate) fn discover_mailcache_dirs() -> anyhow::Result<Vec<String>> {
    #[cfg(target_os = "windows")]
    {
        return Ok(normalize_and_dedupe(discover_windows_mailcache_dirs()));
    }

    #[cfg(target_os = "macos")]
    {
        return Ok(normalize_and_dedupe(discover_macos_mailcache_dirs()));
    }

    #[cfg(not(any(target_os = "windows", target_os = "macos")))]
    {
        Ok(Vec::new())
    }
}

pub(crate) fn path_identity_key(path: &Path) -> String {
    let normalized = fs::canonicalize(path).unwrap_or_else(|_| path.to_path_buf());
    let text = normalized.to_string_lossy().to_string();

    #[cfg(target_os = "windows")]
    {
        return normalize_windows_path_for_display(&text).to_ascii_lowercase();
    }

    #[cfg(not(target_os = "windows"))]
    {
        text
    }
}

#[allow(dead_code)]
fn normalize_and_dedupe(paths: Vec<PathBuf>) -> Vec<String> {
    let mut set = BTreeSet::new();
    for path in paths {
        let normalized = fs::canonicalize(&path).unwrap_or(path);
        let text = normalized.to_string_lossy().to_string();
        #[cfg(target_os = "windows")]
        let text = normalize_windows_path_for_display(&text);
        set.insert(text);
    }
    set.into_iter().collect()
}

#[cfg(target_os = "windows")]
fn discover_windows_mailcache_dirs() -> Vec<PathBuf> {
    discover_windows_from_roots(&windows_drive_roots())
}

#[cfg(target_os = "macos")]
fn discover_macos_mailcache_dirs() -> Vec<PathBuf> {
    let mut bases = Vec::new();
    if let Some(home) = std::env::var_os("HOME") {
        bases.push(PathBuf::from(home).join("Library").join("Containers"));
    }
    bases.push(PathBuf::from("/Library/Containers"));
    discover_macos_in_bases(&bases)
}

#[cfg(any(test, target_os = "windows"))]
fn discover_windows_from_roots(roots: &[PathBuf]) -> Vec<PathBuf> {
    if roots.is_empty() {
        return Vec::new();
    }

    let mut candidates = Vec::new();
    for root in roots {
        for base in WINDOWS_BASE_DIRS {
            collect_windows_candidates_from_base(&root.join(base), &mut candidates);
        }
    }

    let mut valid = candidates
        .into_iter()
        .filter(|path| is_valid_windows_mailcache_dir(path))
        .collect::<Vec<_>>();

    if valid.is_empty() {
        let mut fallback_candidates = Vec::new();
        for root in roots {
            collect_windows_candidates_fallback(root, &mut fallback_candidates);
        }
        valid.extend(
            fallback_candidates
                .into_iter()
                .filter(|path| is_valid_windows_mailcache_dir(path)),
        );
    }

    valid
}

#[cfg(any(test, target_os = "windows"))]
fn collect_windows_candidates_from_base(base: &Path, out: &mut Vec<PathBuf>) {
    if !base.is_dir() {
        return;
    }

    out.push(
        base.join("Rise of Kingdoms Game")
            .join("save")
            .join("mailcache"),
    );

    let Ok(read_dir) = fs::read_dir(base) else {
        return;
    };

    for entry in read_dir.flatten().take(MAX_WINDOWS_BASE_CHILDREN) {
        let Ok(file_type) = entry.file_type() else {
            continue;
        };
        if !file_type.is_dir() {
            continue;
        }
        let child = entry.path();
        out.push(
            child
                .join("Rise of Kingdoms Game")
                .join("save")
                .join("mailcache"),
        );

        // Handle installs with a nested directory before "Rise of Kingdoms Game".
        let Ok(nested_read_dir) = fs::read_dir(&child) else {
            continue;
        };
        for nested_entry in nested_read_dir
            .flatten()
            .take(MAX_WINDOWS_BASE_GRANDCHILDREN)
        {
            let Ok(nested_type) = nested_entry.file_type() else {
                continue;
            };
            if !nested_type.is_dir() {
                continue;
            }
            out.push(
                nested_entry
                    .path()
                    .join("Rise of Kingdoms Game")
                    .join("save")
                    .join("mailcache"),
            );
        }
    }
}

#[cfg(any(test, target_os = "windows"))]
fn collect_windows_candidates_fallback(root: &Path, out: &mut Vec<PathBuf>) {
    if !root.is_dir() {
        return;
    }

    let started = Instant::now();
    let mut visited = 0usize;
    let mut queue = VecDeque::new();
    queue.push_back(root.to_path_buf());

    while let Some(dir) = queue.pop_front() {
        if visited >= MAX_WINDOWS_FALLBACK_DIRS
            || started.elapsed() >= MAX_WINDOWS_FALLBACK_DURATION
        {
            break;
        }
        visited = visited.saturating_add(1);

        let Ok(read_dir) = fs::read_dir(&dir) else {
            continue;
        };

        for entry in read_dir
            .flatten()
            .take(MAX_WINDOWS_FALLBACK_CHILDREN_PER_DIR)
        {
            let Ok(file_type) = entry.file_type() else {
                continue;
            };
            if !file_type.is_dir() || file_type.is_symlink() {
                continue;
            }

            let child = entry.path();
            let Some(name) = child.file_name().and_then(|name| name.to_str()) else {
                continue;
            };

            if name.eq_ignore_ascii_case("Rise of Kingdoms Game") {
                out.push(child.join("save").join("mailcache"));
                continue;
            }

            if !should_skip_windows_dir(name) {
                queue.push_back(child);
            }
        }
    }
}

#[cfg(any(test, target_os = "windows"))]
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

#[cfg(any(test, target_os = "windows"))]
fn is_valid_windows_mailcache_dir(path: &Path) -> bool {
    path_matches_windows_mailcache_suffix(path) && is_valid_mailcache_dir(path)
}

#[cfg(any(test, target_os = "windows"))]
fn path_matches_windows_mailcache_suffix(path: &Path) -> bool {
    let normalized = path
        .to_string_lossy()
        .replace('/', "\\")
        .to_ascii_lowercase();
    normalized.ends_with(WINDOWS_MAILCACHE_SUFFIX_LOWER)
}

#[cfg(any(test, target_os = "macos"))]
fn discover_macos_in_bases(bases: &[PathBuf]) -> Vec<PathBuf> {
    let mut candidates = Vec::new();
    for base in bases {
        if !base.is_dir() {
            continue;
        }

        let Ok(read_dir) = fs::read_dir(base) else {
            continue;
        };

        for entry in read_dir.flatten().take(MAX_MACOS_CONTAINERS) {
            let Ok(file_type) = entry.file_type() else {
                continue;
            };
            if !file_type.is_dir() {
                continue;
            }

            let candidate = entry
                .path()
                .join("Data")
                .join("Documents")
                .join("mailcache");
            if is_valid_macos_mailcache_dir(&candidate) {
                candidates.push(candidate);
            }
        }
    }
    candidates
}

#[cfg(any(test, target_os = "macos"))]
fn is_valid_macos_mailcache_dir(path: &Path) -> bool {
    path_matches_macos_container_pattern(path) && is_valid_mailcache_dir(path)
}

#[cfg(any(test, target_os = "macos"))]
fn path_matches_macos_container_pattern(path: &Path) -> bool {
    let normalized = path.to_string_lossy().replace('\\', "/");
    normalized.ends_with("/Data/Documents/mailcache") && normalized.contains("/Library/Containers/")
}

fn is_valid_mailcache_dir(path: &Path) -> bool {
    if !path.is_dir() {
        return false;
    }
    dir_has_persistent_mail_files(path)
}

fn dir_has_persistent_mail_files(path: &Path) -> bool {
    let Ok(read_dir) = fs::read_dir(path) else {
        return false;
    };
    for entry in read_dir.flatten().take(MAX_MAILCACHE_FILES_TO_CHECK) {
        let Ok(file_type) = entry.file_type() else {
            continue;
        };
        if !file_type.is_file() {
            continue;
        }
        let Some(name) = entry.file_name().to_str().map(str::to_owned) else {
            continue;
        };
        if is_persistent_mail_filename(&name) {
            return true;
        }
    }
    false
}

fn is_persistent_mail_filename(name: &str) -> bool {
    let Some(rest) = name.strip_prefix("Persistent.Mail.") else {
        return false;
    };
    !rest.is_empty() && rest.bytes().all(|byte| byte.is_ascii_digit())
}

#[cfg(target_os = "windows")]
fn windows_drive_roots() -> Vec<PathBuf> {
    let mut roots = Vec::new();
    for letter in b'A'..=b'Z' {
        let root = PathBuf::from(format!("{}:\\", letter as char));
        if root.exists() {
            roots.push(root);
        }
    }
    roots
}

#[cfg(test)]
mod tests {
    use super::{
        discover_macos_in_bases, discover_windows_from_roots, is_persistent_mail_filename,
        normalize_windows_path_for_display, path_matches_windows_mailcache_suffix,
    };
    use std::{fs, path::PathBuf};
    use tempfile::tempdir;

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
        assert!(found.contains(&mailcache));
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
        assert!(found.contains(&standard_mailcache));
        assert!(found.contains(&nested_kr_mailcache));
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
        assert!(found.contains(&deep_mailcache));
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
        assert_eq!(found, vec![mailcache]);
    }

    #[test]
    fn macos_discovery_rejects_non_mailcache_content() {
        let temp = tempdir().expect("tempdir");
        let containers = temp.path().join("Library").join("Containers");
        let mailcache = containers
            .join("com.lilithgame.rok")
            .join("Data")
            .join("Documents")
            .join("mailcache");
        fs::create_dir_all(&mailcache).expect("create dirs");
        fs::write(mailcache.join("README.txt"), b"test").expect("write file");

        let found = discover_macos_in_bases(&[containers]);
        assert!(found.is_empty());
    }
}
