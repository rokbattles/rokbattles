//! Native threads enqueue bounded raw observations only. User/flow authority is
//! checked by the final IPC writer after dequeue, never cached in this queue.
use super::open_lock::OpenLock;
use rokbattles_capture_adapters::Receive;
use rokbattles_capture_ipc::windows::ProtectedInstallation;
use rokbattles_capture_ipc::{Backend, PacketBytes, UnavailableReason};
#[cfg(target_arch = "x86_64")]
use rokbattles_capture_runtime::packet::ClientTcpControl;
use std::{
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
    time::{Duration, Instant},
};
use tokio::sync::{mpsc, watch};

pub const QUEUE_RECORDS: usize = 64;
pub enum NativeRecord {
    Started(Backend),
    Packet(PacketBytes),
    #[cfg(target_arch = "x86_64")]
    ClientControl(ClientTcpControl),
    Unavailable(UnavailableReason),
}
pub struct Pump {
    pub records: mpsc::Receiver<NativeRecord>,
    pub failed: watch::Receiver<bool>,
    stop: Arc<AtomicBool>,
    task: tokio::task::JoinHandle<()>,
}
impl Pump {
    pub fn start() -> Self {
        let (sender, records) = mpsc::channel(QUEUE_RECORDS);
        let (failure, failed) = watch::channel(false);
        let stop = Arc::new(AtomicBool::new(false));
        let stop_copy = Arc::clone(&stop);
        let task = tokio::task::spawn_blocking(move || run(sender, failure, stop_copy));
        Self { records, failed, stop, task }
    }
    pub async fn finish(mut self) {
        self.cancel();
        if !matches!(tokio::time::timeout(Duration::from_secs(5), &mut self.task).await, Ok(Ok(())))
        {
            std::process::abort();
        }
    }
    pub fn cancel(&self) {
        self.stop.store(true, Ordering::Release);
    }
}
impl Drop for Pump {
    fn drop(&mut self) {
        self.cancel();
    }
}

fn send(
    sender: &mpsc::Sender<NativeRecord>,
    failure: &watch::Sender<bool>,
    stop: &AtomicBool,
    record: NativeRecord,
) -> bool {
    if stop.load(Ordering::Acquire) {
        return false;
    }
    if matches!(&record, NativeRecord::Packet(bytes) if bytes.len() > rokbattles_capture_ipc::MAX_BODY_BYTES)
    {
        let _outcome = failure.send(true);
        stop.store(true, Ordering::Release);
        return false;
    }
    if sender.try_send(record).is_err() {
        // RAII wipes any packet rejected by the queue. No post-loss record follows.
        let _outcome = failure.send(true);
        stop.store(true, Ordering::Release);
        return false;
    }
    true
}
fn unavailable(sender: &mpsc::Sender<NativeRecord>, reason: UnavailableReason) {
    let _outcome = sender.try_send(NativeRecord::Unavailable(reason));
}

fn run(sender: mpsc::Sender<NativeRecord>, failure: watch::Sender<bool>, stop: Arc<AtomicBool>) {
    if stop.load(Ordering::Acquire) {
        return;
    }
    let Ok(lock) = OpenLock::acquire() else {
        unavailable(&sender, UnavailableReason::Busy);
        return;
    };
    let Ok(_ready) = ProtectedInstallation::maintenance_ready() else {
        unavailable(&sender, UnavailableReason::HelperNotProvisioned);
        return;
    };
    if stop.load(Ordering::Acquire) {
        return;
    }
    #[cfg(target_arch = "x86_64")]
    {
        use rokbattles_capture_adapters::windivert::WinDivert;
        if let Ok(trust) = super::native_trust::TrustedWinDivert::verify() {
            if stop.load(Ordering::Acquire) {
                return;
            }
            // SAFETY: fixed protected x64 path, compiled exact size/hash pins and
            // cached nested driver signature; immutable handles outlive DLL use.
            if let Ok(backend) = unsafe { WinDivert::load(trust.dll_path()) }
                && !stop.load(Ordering::Acquire)
                && let Ok(capture) = backend.open()
            {
                drop(lock);
                if send(&sender, &failure, &stop, NativeRecord::Started(Backend::WinDivert)) {
                    run_windivert(&capture, &sender, &failure, &stop);
                }
                return;
            }
        }
    }
    // Windows ARM64 and x64 fallback use only a separately installed, already
    // running Npcap driver. The library is never linked, bundled or installed here.
    run_npcap(lock, &sender, &failure, &stop);
}

#[cfg(target_arch = "x86_64")]
fn run_windivert(
    capture: &rokbattles_capture_adapters::windivert::Capture<'_>,
    sender: &mpsc::Sender<NativeRecord>,
    failure: &watch::Sender<bool>,
    stop: &AtomicBool,
) {
    std::thread::scope(|scope| {
        let receiver = scope.spawn(|| {
            while !stop.load(Ordering::Acquire) {
                let record = match capture.receive() {
                    Ok(Receive::Packet(bytes)) => NativeRecord::Packet(bytes.into()),
                    Ok(Receive::ClientControl(control)) => NativeRecord::ClientControl(control),
                    Ok(Receive::Idle | Receive::Discarded) => continue,
                    Ok(Receive::End) | Err(_) => break,
                };
                if !send(sender, failure, stop, record) {
                    return;
                }
            }
        });
        while !stop.load(Ordering::Acquire) && !receiver.is_finished() {
            std::thread::sleep(Duration::from_millis(20));
        }
        stop.store(true, Ordering::Release);
        let _shutdown = capture.shutdown();
        if receiver.join().is_err() {
            std::process::abort();
        }
    });
}

fn run_npcap(
    lock: OpenLock,
    sender: &mpsc::Sender<NativeRecord>,
    failure: &watch::Sender<bool>,
    stop: &AtomicBool,
) {
    use rokbattles_capture_adapters::pcap::Pcap;
    if stop.load(Ordering::Acquire) {
        return;
    }
    let Ok((library, driver)) = ProtectedInstallation::installed_npcap() else {
        unavailable(sender, UnavailableReason::NativeBackend);
        return;
    };
    if super::native_trust::require_running_driver("npcap", driver.path()).is_err() {
        unavailable(sender, UnavailableReason::NativeBackend);
        return;
    }
    // SAFETY: fixed separately installed Npcap system path and protected native
    // dependencies/driver, no user-writable ancestor or loader search location.
    let Ok(backend) = (unsafe { Pcap::load(library.path()) }) else {
        unavailable(sender, UnavailableReason::NativeBackend);
        return;
    };
    let Ok(interfaces) = crate::interfaces::enumerate() else {
        unavailable(sender, UnavailableReason::NativeBackend);
        return;
    };
    if interfaces.is_empty() {
        unavailable(sender, UnavailableReason::NativeBackend);
        return;
    }
    let mut captures = Vec::new();
    for interface in &interfaces {
        if stop.load(Ordering::Acquire) {
            return;
        }
        let Ok(mut capture) = backend.open(&interface.name, &interface.addresses) else {
            unavailable(sender, UnavailableReason::NativeBackend);
            return;
        };
        if capture.check_no_packet_loss().is_err() {
            unavailable(sender, UnavailableReason::NativeBackend);
            return;
        }
        captures.push(capture);
    }
    let mut original: Vec<_> = interfaces
        .iter()
        .map(|interface| (interface.name.clone(), interface.addresses.clone()))
        .collect();
    original.sort();
    drop(lock);
    if !send(sender, failure, stop, NativeRecord::Started(Backend::Pcap)) {
        return;
    }
    let mut checked = Instant::now();
    while !stop.load(Ordering::Acquire) {
        if checked.elapsed() >= Duration::from_secs(2) {
            let Ok(current) = crate::interfaces::enumerate() else {
                break;
            };
            let mut current: Vec<_> = current
                .into_iter()
                .map(|interface| (interface.name, interface.addresses))
                .collect();
            current.sort();
            if current != original {
                break;
            } // transport ends; reconnect obtains a fresh baseline.
            checked = Instant::now();
        }
        for capture in &mut captures {
            if capture.check_no_packet_loss().is_err() {
                let _sent = failure.send(true);
                return;
            }
            let record = match capture.receive() {
                Ok(Receive::Packet(bytes)) => Some(NativeRecord::Packet(bytes.into())),
                Ok(Receive::Idle | Receive::Discarded) => None,
                Ok(Receive::ClientControl(_) | Receive::End) | Err(_) => return,
            };
            if capture.check_no_packet_loss().is_err() {
                let _sent = failure.send(true);
                return;
            }
            if let Some(record) = record
                && !send(sender, failure, stop, record)
            {
                return;
            }
        }
        std::thread::sleep(Duration::from_millis(2));
    }
}
