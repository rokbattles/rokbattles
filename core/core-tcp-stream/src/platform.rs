//! Short packet-capture hints for UI and diagnostics.

/// Runtime hint for opening capture devices on this platform.
pub fn runtime_hint() -> &'static str {
    if cfg!(target_os = "windows") {
        "Windows runtime prerequisite: install Npcap with WinPcap API-compatible mode enabled, then restart the app."
    } else if cfg!(target_os = "linux") {
        "Linux runtime prerequisite: run with packet-capture permissions, for example cap_net_raw and cap_net_admin on the app binary."
    } else if cfg!(target_os = "macos") {
        "macOS runtime prerequisite: grant access to /dev/bpf* capture devices."
    } else {
        "Runtime prerequisite: install libpcap/Npcap for this platform and grant packet-capture permissions."
    }
}
