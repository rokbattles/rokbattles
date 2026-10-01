//! Per-user privileged Unix broker. Explicitly installed system services only;
//! no install/elevation, user paths, payload decoder, HTTP or storage surface.
mod agent;
mod bootstrap;
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

/// One aggregate bound spans all process and FD enumeration in a fresh lookup.
struct WorkBudget {
    remaining: u32,
}
impl WorkBudget {
    fn new() -> Self {
        Self { remaining: 32_768 }
    }
    fn consume(&mut self, amount: u32) -> std::io::Result<()> {
        self.remaining = self.remaining.checked_sub(amount).ok_or_else(|| {
            std::io::Error::new(std::io::ErrorKind::OutOfMemory, "ownership work budget exhausted")
        })?;
        Ok(())
    }
}
#[cfg(test)]
mod budget_tests {
    #[test]
    fn every_process_and_descriptor_share_one_fail_closed_limit() {
        let mut budget = super::WorkBudget::new();
        budget.consume(1).expect("process");
        budget.consume(32_767).expect("descriptors");
        budget.consume(1).expect_err("aggregate limit");
        assert_eq!(budget.remaining, 0);
    }
}
