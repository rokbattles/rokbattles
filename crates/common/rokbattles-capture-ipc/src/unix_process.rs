//! Process-local capture privacy: never let a crash persist raw packet buffers.
//! Called by the helper and unprivileged agent before starting capture workers.
use std::io;

fn denied() -> io::Error {
    io::Error::new(io::ErrorKind::PermissionDenied, "capture process core protection unavailable")
}
trait ProcessPolicy {
    fn set_core_zero(&mut self) -> io::Result<()>;
    fn core_is_zero(&mut self) -> io::Result<bool>;
    fn disable_platform_dumps(&mut self) -> io::Result<()>;
}
fn enforce(policy: &mut impl ProcessPolicy) -> io::Result<()> {
    policy.set_core_zero()?;
    if !policy.core_is_zero()? {
        return Err(denied());
    }
    policy.disable_platform_dumps()
}
struct NativePolicy;
impl ProcessPolicy for NativePolicy {
    fn set_core_zero(&mut self) -> io::Result<()> {
        let limit = libc::rlimit { rlim_cur: 0, rlim_max: 0 };
        // SAFETY: valid immutable limit; affects only this process, before workers.
        if unsafe { libc::setrlimit(libc::RLIMIT_CORE, &limit) } != 0 {
            return Err(io::Error::last_os_error());
        }
        Ok(())
    }
    fn core_is_zero(&mut self) -> io::Result<bool> {
        let mut limit = libc::rlimit { rlim_cur: 0, rlim_max: 0 };
        // SAFETY: correctly sized writable native output, process-local query.
        if unsafe { libc::getrlimit(libc::RLIMIT_CORE, &mut limit) } != 0 {
            return Err(io::Error::last_os_error());
        }
        Ok(limit.rlim_cur == 0 && limit.rlim_max == 0)
    }
    fn disable_platform_dumps(&mut self) -> io::Result<()> {
        #[cfg(target_os = "linux")]
        {
            let zero: libc::c_ulong = 0;
            // RLIMIT_CORE alone is ignored by Linux piped core collectors.
            // SAFETY: process-local prctl option, integral arguments, no pointers.
            if unsafe { libc::prctl(libc::PR_SET_DUMPABLE, zero, zero, zero, zero) } != 0 {
                return Err(io::Error::last_os_error());
            }
            // SAFETY: read-only query for this process, no pointer arguments.
            if unsafe { libc::prctl(libc::PR_GET_DUMPABLE, zero, zero, zero, zero) } != 0 {
                return Err(denied());
            }
        }
        Ok(())
    }
}

/// Set and verify zero soft/hard core limits on Linux/macOS; on Linux also
/// disable dumpability for piped core handlers. Never changes host policy.
/// Failure must prevent agent/helper startup. Call before creating workers.
pub fn harden_capture_process() -> io::Result<()> {
    enforce(&mut NativePolicy)
}

#[cfg(test)]
mod tests {
    use super::*;
    struct Policy {
        fail_set: bool,
        zero: bool,
        fail_platform: bool,
        calls: Vec<&'static str>,
    }
    impl ProcessPolicy for Policy {
        fn set_core_zero(&mut self) -> io::Result<()> {
            self.calls.push("set");
            if self.fail_set { Err(denied()) } else { Ok(()) }
        }
        fn core_is_zero(&mut self) -> io::Result<bool> {
            self.calls.push("verify");
            Ok(self.zero)
        }
        fn disable_platform_dumps(&mut self) -> io::Result<()> {
            self.calls.push("platform");
            if self.fail_platform { Err(denied()) } else { Ok(()) }
        }
    }
    #[test]
    fn core_limits_and_platform_protection_are_verified_before_success() {
        let mut policy =
            Policy { fail_set: false, zero: true, fail_platform: false, calls: Vec::new() };
        enforce(&mut policy).expect("synthetic protected process");
        assert_eq!(policy.calls, ["set", "verify", "platform"]);
    }
    #[test]
    fn failed_or_unverified_limits_never_allow_startup() {
        for (fail_set, zero, fail_platform, expected) in [
            (true, true, false, vec!["set"]),
            (false, false, false, vec!["set", "verify"]),
            (false, true, true, vec!["set", "verify", "platform"]),
        ] {
            let mut policy = Policy { fail_set, zero, fail_platform, calls: Vec::new() };
            enforce(&mut policy).expect_err("fail closed");
            assert_eq!(policy.calls, expected);
        }
    }
}
