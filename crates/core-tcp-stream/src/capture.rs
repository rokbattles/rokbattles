//! Live packet capture through libpcap/Npcap.

use std::{
    thread,
    time::{Duration, Instant},
};

use pcap::{Active, Capture, Device};

use crate::{
    packet::parse_tcp_packet,
    platform,
    tracker::{StreamTracker, TrackerEvent},
};

const AUTO_POLL_SLEEP: Duration = Duration::from_millis(25);
const WAITING_EVENT_INTERVAL: Duration = Duration::from_secs(1);

/// Where live capture should read from.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CaptureSource {
    /// Read from a specific pcap device name.
    Device(String),
    /// Probe devices and use the first one that sees a usable stream.
    Auto {
        /// How long to probe before trying the device list again.
        probe_duration: Duration,
    },
}

/// Options for live stream capture.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CaptureConfig {
    /// Device selection mode.
    pub source: CaptureSource,
    /// Remote TCP port to watch.
    pub port: u16,
}

/// Events from live capture.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CaptureEvent {
    /// Capture chose a device for this run.
    DeviceSelected {
        /// pcap device name.
        name: String,
    },
    /// No processable client stream has been seen yet.
    WaitingForClient,
    /// Event from the stream tracker.
    Tracker(TrackerEvent),
}

/// Failures while opening devices or reading live traffic.
#[derive(Debug, thiserror::Error)]
pub enum CaptureError {
    /// Caller requested shutdown.
    #[error("capture stopped")]
    Stopped,
    /// pcap could not list capture devices.
    #[error("failed to list capture devices")]
    DeviceList {
        /// Original pcap error.
        source: pcap::Error,
    },
    /// Requested device name was not found.
    #[error("capture device `{name}` was not found")]
    DeviceNotFound {
        /// Requested pcap device name.
        name: String,
    },
    /// pcap could not open a device.
    #[error("failed to open capture device `{name}`")]
    DeviceOpen {
        /// pcap device name.
        name: String,
        /// Original pcap error.
        source: pcap::Error,
    },
    /// Auto-probing could not open any capture device.
    #[error("no capture devices could be opened")]
    NoUsableDevice {
        /// Failure details for each device we tried.
        failures: Vec<DeviceOpenFailure>,
    },
    /// pcap could not install the TCP port filter.
    #[error("failed to install tcp port {port} capture filter")]
    FilterInstall {
        /// TCP port used in the filter.
        port: u16,
        /// Original pcap error.
        source: pcap::Error,
    },
    /// pcap failed while reading live traffic.
    #[error("live capture failed")]
    LiveCapture {
        /// Original pcap error.
        source: pcap::Error,
    },
}

impl CaptureError {
    /// Short platform-specific hint for the UI.
    pub fn user_hint(&self) -> Option<&'static str> {
        match self {
            Self::DeviceList { .. }
            | Self::DeviceOpen { .. }
            | Self::NoUsableDevice { .. }
            | Self::LiveCapture { .. } => Some(platform::runtime_hint()),
            Self::Stopped => None,
            Self::DeviceNotFound { .. } => {
                Some("Select a capture device visible to pcap, or use automatic detection.")
            }
            Self::FilterInstall { .. } => None,
        }
    }

    /// Per-device failures from auto-probing.
    pub fn details(&self) -> &[DeviceOpenFailure] {
        match self {
            Self::NoUsableDevice { failures } => failures,
            _ => &[],
        }
    }
}

/// Why one capture device could not be opened.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DeviceOpenFailure {
    /// pcap device name.
    pub name: String,
    /// Human-readable pcap error.
    pub message: String,
}

/// List capture devices visible to libpcap/Npcap.
pub fn list_devices() -> Result<Vec<Device>, CaptureError> {
    Device::list().map_err(|source| CaptureError::DeviceList { source })
}

/// Run live capture until interrupted or until pcap reports a fatal error.
///
/// # Errors
///
/// Returns [`CaptureError`] when setup fails or live capture stops with an
/// unrecoverable pcap error.
pub fn run_capture(
    config: CaptureConfig,
    mut on_event: impl FnMut(CaptureEvent),
) -> Result<(), CaptureError> {
    run_capture_until(config, &mut on_event, || true)
}

/// Run live capture until `should_continue` returns false or pcap fails.
///
/// # Errors
///
/// Returns [`CaptureError::Stopped`] for requested shutdown. Other variants are
/// pcap setup or read failures.
pub fn run_capture_until(
    config: CaptureConfig,
    mut on_event: impl FnMut(CaptureEvent),
    mut should_continue: impl FnMut() -> bool,
) -> Result<(), CaptureError> {
    let (device, mut capture, mut tracker, pending_events) =
        open_capture(config, &mut should_continue)?;
    on_event(CaptureEvent::DeviceSelected { name: device.name });
    if pending_events.is_empty() {
        on_event(CaptureEvent::WaitingForClient);
    } else {
        let accepted =
            pending_events.iter().any(|event| matches!(event, TrackerEvent::StreamAccepted { .. }));
        for event in pending_events {
            on_event(CaptureEvent::Tracker(event));
        }
        if !accepted {
            on_event(CaptureEvent::WaitingForClient);
        }
    }

    let link_type = capture.get_datalink().0;
    let mut last_waiting_event = Instant::now();
    loop {
        if !should_continue() {
            return Err(CaptureError::Stopped);
        }
        match capture.next_packet() {
            Ok(packet) => {
                if let Some(payload) = parse_tcp_packet(link_type, packet.data) {
                    for event in tracker.push_packet(payload) {
                        on_event(CaptureEvent::Tracker(event));
                    }
                }
            }
            Err(pcap::Error::TimeoutExpired) => {
                if last_waiting_event.elapsed() >= WAITING_EVENT_INTERVAL {
                    on_event(CaptureEvent::WaitingForClient);
                    last_waiting_event = Instant::now();
                }
                thread::sleep(AUTO_POLL_SLEEP);
            }
            Err(source) => return Err(CaptureError::LiveCapture { source }),
        }
    }
}

fn open_capture(
    config: CaptureConfig,
    should_continue: &mut impl FnMut() -> bool,
) -> Result<(Device, Capture<Active>, StreamTracker, Vec<TrackerEvent>), CaptureError> {
    match config.source {
        CaptureSource::Device(name) => {
            let device = list_devices()?
                .into_iter()
                .find(|device| device.name == name)
                .ok_or(CaptureError::DeviceNotFound { name })?;
            let capture = open_device(device.clone(), config.port, false)?;
            Ok((device, capture, StreamTracker::new(config.port), Vec::new()))
        }
        CaptureSource::Auto { probe_duration } => {
            open_auto_capture(config.port, probe_duration, should_continue)
        }
    }
}

fn open_device(
    device: Device,
    port: u16,
    nonblocking: bool,
) -> Result<Capture<Active>, CaptureError> {
    let name = device.name.clone();
    let mut capture = Capture::from_device(device)
        .map_err(|source| CaptureError::DeviceOpen { name: name.clone(), source })?
        .promisc(false)
        .immediate_mode(true)
        .timeout(1_000)
        .open()
        .map_err(|source| CaptureError::DeviceOpen { name: name.clone(), source })?;
    capture
        .filter(&format!("tcp port {port}"), true)
        .map_err(|source| CaptureError::FilterInstall { port, source })?;
    if nonblocking {
        capture =
            capture.setnonblock().map_err(|source| CaptureError::DeviceOpen { name, source })?;
    }
    Ok(capture)
}

fn open_auto_capture(
    port: u16,
    retry_duration: Duration,
    should_continue: &mut impl FnMut() -> bool,
) -> Result<(Device, Capture<Active>, StreamTracker, Vec<TrackerEvent>), CaptureError> {
    loop {
        if !should_continue() {
            return Err(CaptureError::Stopped);
        }
        let mut candidates = open_probe_candidates(port)?;
        if candidates.is_empty() {
            thread::sleep(retry_duration.max(WAITING_EVENT_INTERVAL));
            continue;
        }

        loop {
            if !should_continue() {
                return Err(CaptureError::Stopped);
            }
            let mut index = 0;
            while index < candidates.len() {
                if !should_continue() {
                    return Err(CaptureError::Stopped);
                }
                match poll_probe_candidate(&mut candidates[index]) {
                    Ok(ProbeStatus::Selected) => {
                        let candidate = candidates.swap_remove(index);
                        return Ok((
                            candidate.device,
                            candidate.capture,
                            candidate.tracker,
                            candidate.pending_events,
                        ));
                    }
                    Ok(ProbeStatus::Idle) => {
                        index += 1;
                    }
                    Err(_) => {
                        candidates.swap_remove(index);
                    }
                }
            }

            if candidates.is_empty() {
                break;
            }

            thread::sleep(AUTO_POLL_SLEEP);
        }
    }
}

fn open_probe_candidates(port: u16) -> Result<Vec<ProbeCandidate>, CaptureError> {
    let mut candidates = Vec::new();
    let mut failures = Vec::new();
    for device in list_devices()? {
        match open_device(device.clone(), port, true) {
            Ok(capture) => {
                let link_type = capture.get_datalink().0;
                candidates.push(ProbeCandidate {
                    device,
                    capture,
                    link_type,
                    tracker: StreamTracker::new(port),
                    pending_events: Vec::new(),
                });
            }
            Err(error) => failures.push(DeviceOpenFailure {
                name: device.name,
                message: device_probe_failure_message(&error),
            }),
        }
    }

    if candidates.is_empty() && !failures.is_empty() {
        return Err(CaptureError::NoUsableDevice { failures });
    }

    Ok(candidates)
}

fn device_probe_failure_message(error: &CaptureError) -> String {
    match error {
        CaptureError::DeviceOpen { source, .. } | CaptureError::FilterInstall { source, .. } => {
            source.to_string()
        }
        _ => error.to_string(),
    }
}

fn poll_probe_candidate(candidate: &mut ProbeCandidate) -> Result<ProbeStatus, CaptureError> {
    loop {
        match candidate.capture.next_packet() {
            Ok(packet) => {
                let Some(payload) = parse_tcp_packet(candidate.link_type, packet.data) else {
                    continue;
                };
                let events = candidate.tracker.push_packet(payload);
                let selected = events.iter().any(|event| {
                    matches!(
                        event,
                        TrackerEvent::StreamAccepted { .. } | TrackerEvent::StreamIgnored { .. }
                    )
                });
                candidate.pending_events.extend(events);
                if selected {
                    return Ok(ProbeStatus::Selected);
                }
            }
            Err(pcap::Error::TimeoutExpired) => return Ok(ProbeStatus::Idle),
            Err(source) => return Err(CaptureError::LiveCapture { source }),
        }
    }
}

struct ProbeCandidate {
    device: Device,
    capture: Capture<Active>,
    link_type: i32,
    tracker: StreamTracker,
    pending_events: Vec<TrackerEvent>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ProbeStatus {
    Idle,
    Selected,
}
