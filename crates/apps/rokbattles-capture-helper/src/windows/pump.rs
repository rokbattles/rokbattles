use rokbattles_capture_ipc::{Record, UnavailableReason, windows::Identity};
use std::{
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
    time::Duration,
};
use tokio::sync::{mpsc, watch};

pub const QUEUE_RECORDS: usize = 64; // <= 64 * 65,535 body bytes, plus fixed metadata.

pub struct Pump {
    pub records: mpsc::Receiver<Record>,
    pub failed: watch::Receiver<bool>,
    stop: Arc<AtomicBool>,
    task: tokio::task::JoinHandle<()>,
}
impl Pump {
    pub fn start(identity: Identity) -> Self {
        let (sender, records) = mpsc::channel(QUEUE_RECORDS);
        let (failure, failed) = watch::channel(false);
        let stop = Arc::new(AtomicBool::new(false));
        let stop_copy = Arc::clone(&stop);
        let task = tokio::task::spawn_blocking(move || run(identity, sender, failure, stop_copy));
        Self { records, failed, stop, task }
    }
    pub async fn finish(mut self) {
        self.stop.store(true, Ordering::Release);
        // WinDivertShutdown wakes the native receiver; timeout never keeps a pipe
        // or opted-in session alive. Process-level service shutdown also terminates it.
        if !matches!(tokio::time::timeout(Duration::from_secs(5), &mut self.task).await, Ok(Ok(())))
        {
            // A timeout must not masquerade as a completed service stop/update.
            std::process::abort();
        }
    }
    pub fn cancel(&self) {
        self.stop.store(true, Ordering::Release);
    }
}
impl Drop for Pump {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::Release);
    }
}

#[cfg(target_arch = "x86_64")]
fn run(
    _identity: Identity,
    sender: mpsc::Sender<Record>,
    failure: watch::Sender<bool>,
    stop: Arc<AtomicBool>,
) {
    use super::{native_trust::TrustedWinDivert, open_lock::OpenLock};
    use rokbattles_capture_adapters::{Receive, windivert::WinDivert};
    let opened = (|| {
        let lock = OpenLock::acquire().map_err(|_error| ())?;
        let ready = rokbattles_capture_ipc::windows::ProtectedInstallation::maintenance_ready()
            .map_err(|_error| ())?;
        let trust = TrustedWinDivert::verify().map_err(|_error| ())?;
        // SAFETY: fixed protected architecture path, exact compiled byte/hash pins,
        // signed driver with exact nested DRIVER_ACTION_VERIFY; pinned files remain
        // immutable while the DLL is loaded. No user-selected native path exists.
        let backend = unsafe { WinDivert::load(trust.dll_path()) }.map_err(|_error| ())?;
        Ok::<_, ()>((lock, ready, trust, backend))
    })();
    let Ok((lock, _ready, _trust, backend)) = opened else {
        let _outcome = sender.try_send(Record::Unavailable(UnavailableReason::NativeBackend));
        return;
    };
    let Ok(capture) = backend.open() else {
        let _outcome = sender.try_send(Record::Unavailable(UnavailableReason::NativeBackend));
        return;
    };
    drop(lock); // NO_INSTALL open finished under the shared cross-process gate.
    if stop.load(Ordering::Acquire) || sender.try_send(Record::Started).is_err() {
        return;
    }
    std::thread::scope(|scope| {
        let receiver = scope.spawn(|| {
            while !stop.load(Ordering::Acquire) {
                let records = match capture.receive() {
                    Ok(Receive::Packet(bytes)) => vec![Record::ServerPacket(bytes.into())],
                    Ok(Receive::ClientControl(control)) => vec![Record::ClientControl(control)],
                    Ok(Receive::Idle | Receive::Discarded) => continue,
                    Ok(Receive::End) | Err(_) => break,
                };
                for record in records {
                    if sender.try_send(record).is_err() {
                        // Do not block a capture thread behind a slow client. No
                        // post-loss record is queued; close the authenticated pipe.
                        let _outcome = failure.send(true);
                        stop.store(true, Ordering::Release);
                        return;
                    }
                }
            }
        });
        while !stop.load(Ordering::Acquire) && !receiver.is_finished() {
            std::thread::sleep(Duration::from_millis(20));
        }
        stop.store(true, Ordering::Release);
        let _outcome = capture.shutdown();
        let _outcome = receiver.join();
    });
}

#[cfg(not(target_arch = "x86_64"))]
fn run(
    _identity: Identity,
    sender: mpsc::Sender<Record>,
    _failure: watch::Sender<bool>,
    _stop: Arc<AtomicBool>,
) {
    let _outcome = sender.try_send(Record::Unavailable(UnavailableReason::OwnershipUnavailable));
}
