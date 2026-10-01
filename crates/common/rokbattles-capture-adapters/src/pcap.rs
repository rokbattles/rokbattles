//! Runtime pcap adapter for supported 64-bit desktop targets. No native link dependency.

#[cfg(all(
    any(target_os = "linux", target_os = "macos", target_os = "windows"),
    any(target_arch = "x86_64", target_arch = "aarch64")
))]
mod native;
#[cfg(all(
    any(target_os = "linux", target_os = "macos", target_os = "windows"),
    any(target_arch = "x86_64", target_arch = "aarch64")
))]
pub use native::{Capture, Pcap};

#[cfg(not(all(
    any(target_os = "linux", target_os = "macos", target_os = "windows"),
    any(target_arch = "x86_64", target_arch = "aarch64")
)))]
pub struct Pcap;

#[cfg(not(all(
    any(target_os = "linux", target_os = "macos", target_os = "windows"),
    any(target_arch = "x86_64", target_arch = "aarch64")
)))]
impl Pcap {
    /// Always returns an unsupported-platform error without loading a library.
    ///
    /// # Safety
    /// Matches the supported-platform API; no native code is run here.
    pub unsafe fn load(_path: &std::path::Path) -> Result<Self, crate::Error> {
        Err(crate::Error::UnsupportedPlatform("pcap (Windows/Linux/macOS x86_64/aarch64)"))
    }
}
