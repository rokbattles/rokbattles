//! Runtime WinDivert 2.x adapter, supported only on Windows x86/x64.

#[cfg(all(windows, any(target_arch = "x86", target_arch = "x86_64")))]
mod native;
#[cfg(all(windows, any(target_arch = "x86", target_arch = "x86_64")))]
pub use native::{Capture, WinDivert};

#[cfg(not(all(windows, any(target_arch = "x86", target_arch = "x86_64"))))]
pub struct WinDivert;

#[cfg(not(all(windows, any(target_arch = "x86", target_arch = "x86_64"))))]
impl WinDivert {
    /// Always returns an unsupported-platform error without loading a library.
    ///
    /// # Safety
    /// Matches the supported-platform API; no native code is run here.
    pub unsafe fn load(_path: &std::path::Path) -> Result<Self, crate::Error> {
        Err(crate::Error::UnsupportedPlatform("WinDivert (Windows x86/x64)"))
    }
}

#[cfg(test)]
mod tests {
    #[cfg(not(all(windows, any(target_arch = "x86", target_arch = "x86_64"))))]
    #[test]
    fn unsupported_platform_does_not_load_a_dll() {
        // SAFETY: this platform's implementation never invokes native code.
        let result = unsafe { super::WinDivert::load(std::path::Path::new("WinDivert.dll")) };
        assert!(matches!(result, Err(crate::Error::UnsupportedPlatform(_))));
    }
}
