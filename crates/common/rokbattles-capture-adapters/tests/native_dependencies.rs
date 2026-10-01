//! Explicit native-library acceptance probes, separate from synthetic tests.
//!
//! These tests only load trusted libraries and resolve the production adapter's
//! symbols. They never create a pcap handle, open WinDivert, enumerate interfaces,
//! start a service, or receive packets. Run one exact ignored test at a time after
//! the platform-specific preparation in `docs/desktop-capture.md`.

use std::path::Path;

use rokbattles_capture_adapters::{Error, pcap::Pcap, windivert::WinDivert};

#[test]
fn missing_native_libraries_are_recoverable() {
    // A regular source file cannot also be the parent directory of a library.
    let missing = Path::new(env!("CARGO_MANIFEST_DIR")).join("Cargo.toml/missing-library");
    // SAFETY: canonicalization fails before any native code can be loaded.
    let pcap = unsafe { Pcap::load(&missing) };
    assert!(matches!(pcap, Err(Error::Library { .. }) | Err(Error::UnsupportedPlatform(_))));
    // SAFETY: the path cannot exist; unsupported targets also return before load.
    let windivert = unsafe { WinDivert::load(&missing) };
    assert!(matches!(windivert, Err(Error::Library { .. }) | Err(Error::UnsupportedPlatform(_))));
}

#[cfg(all(
    any(target_os = "linux", target_os = "macos"),
    any(target_arch = "x86_64", target_arch = "aarch64")
))]
#[test]
#[ignore = "requires the trusted OS libpcap runtime; run this exact load-only probe explicitly"]
fn pcap_system_library_loads_without_capture() -> Result<(), Box<dyn std::error::Error>> {
    #[cfg(all(target_os = "linux", target_arch = "x86_64"))]
    let path = Path::new("/usr/lib/x86_64-linux-gnu/libpcap.so.0.8");
    #[cfg(all(target_os = "linux", target_arch = "aarch64"))]
    let path = Path::new("/usr/lib/aarch64-linux-gnu/libpcap.so.0.8");
    #[cfg(target_os = "macos")]
    let path = Path::new("/usr/lib/libpcap.A.dylib");

    // SAFETY: explicitly invoked only on a trusted runner after Ubuntu's signed
    // package installation, or against Apple's exact SIP-protected dyld image.
    // No environment-selected library or capture operation is accepted here.
    let library = unsafe { Pcap::load(path) }?;
    drop(library);
    Ok(())
}

#[cfg(all(windows, target_arch = "x86_64"))]
#[test]
#[ignore = "requires pinned WinDivert files and successful Windows driver signature verification"]
fn windivert_verified_library_loads_without_capture() -> Result<(), Box<dyn std::error::Error>> {
    let path = Path::new(env!("CARGO_MANIFEST_DIR")).join(
        "../../apps/rokbattles-desktop/vendor/windivert/x86_64-pc-windows-msvc/WinDivert.dll",
    );
    // SAFETY: the CI preparation verifies the committed archive/member hashes
    // and upstream driver signatures immediately before this exact ignored test.
    // Load resolves functions only; WinDivertOpen and service APIs are never called.
    let library = unsafe { WinDivert::load(&path) }?;
    drop(library);
    Ok(())
}

#[cfg(windows)]
#[test]
#[ignore = "manual only: administrator-installed, license-approved and signature-verified Npcap"]
fn npcap_preinstalled_library_loads_without_capture() -> Result<(), Box<dyn std::error::Error>> {
    let path = std::env::var_os("ROKBATTLES_VERIFIED_NPCAP")
        .ok_or("run the documented Npcap verification script first")?;
    // SAFETY: the manual verification script selects only native System32/Npcap,
    // checks both DLL publisher signatures, then invokes this exact ignored test.
    // The operator has already installed and accepted Npcap separately. No capture
    // handle or pcap_init call occurs during Pcap::load.
    let library = unsafe { Pcap::load(Path::new(&path)) }?;
    drop(library);
    Ok(())
}
