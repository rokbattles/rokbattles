//! Per-user privileged Unix broker. Explicitly installed system services only;
//! no install/elevation, user paths, payload decoder, HTTP or storage surface.
mod interfaces;
#[cfg(target_os = "linux")]
mod linux;
#[cfg(target_os = "macos")]
mod macos;
mod pump;
mod service;
mod trust;
#[cfg(target_os = "linux")]
use linux::{ProcessIdentity, UnixOwnerLookup};
#[cfg(target_os = "macos")]
use macos::{ProcessIdentity, UnixOwnerLookup};
pub use service::dispatch;

/// Administrative instance UID is the only parameter accepted. It is never
/// trusted as a peer credential; kernel UID/PID authentication is still required.
pub fn service_uid(arguments: &[std::ffi::OsString]) -> Option<u32> {
    if arguments.len() != 3
        || arguments.first()?.to_str()? != "--service"
        || arguments.get(1)?.to_str()? != "--uid"
    {
        return None;
    }
    let value = arguments.get(2)?.to_str()?;
    if value.is_empty()
        || value.len() > 10
        || !value.bytes().all(|byte| byte.is_ascii_digit())
        || (value.len() > 1 && value.starts_with('0'))
    {
        return None;
    }
    let uid: u32 = value.parse().ok()?;
    (uid != 0 && uid != u32::MAX).then_some(uid)
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn service_arguments_allow_only_a_canonical_nonroot_uid() {
        let args = |uid: &str| ["--service", "--uid", uid].map(std::ffi::OsString::from);
        assert_eq!(service_uid(&args("501")), Some(501));
        for uid in [
            "0",
            "00",
            "0501",
            "-1",
            "+501",
            "4294967295",
            "4294967296",
            "1;id",
            "$(id)",
            "1000/../1001",
            "",
        ] {
            assert_eq!(service_uid(&args(uid)), None);
        }
        assert_eq!(service_uid(&["--service".into()]), None);
    }
}
