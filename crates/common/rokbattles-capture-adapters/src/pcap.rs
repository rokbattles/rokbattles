//! Runtime libpcap adapter for Linux and macOS. No libpcap link dependency.

#[cfg(any(target_os = "linux", target_os = "macos"))]
mod native;
#[cfg(any(target_os = "linux", target_os = "macos"))]
pub use native::{Capture, Pcap};

#[cfg(not(any(target_os = "linux", target_os = "macos")))]
pub struct Pcap;

#[cfg(not(any(target_os = "linux", target_os = "macos")))]
impl Pcap {
    /// Always returns an unsupported-platform error without loading a library.
    ///
    /// # Safety
    /// Matches the supported-platform API; no native code is run here.
    pub unsafe fn load(_path: &std::path::Path) -> Result<Self, crate::Error> {
        Err(crate::Error::UnsupportedPlatform("pcap (Linux/macOS)"))
    }
}
