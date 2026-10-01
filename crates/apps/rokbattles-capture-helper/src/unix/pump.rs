//! Native pcap stays on one thread. Queues carry bounded raw records, never
//! ownership authority. Any source loss closes the session before more packets.
use rokbattles_capture_ipc::{Record, UnavailableReason};
use std::{
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
    time::{Duration, Instant},
};
use tokio::sync::{mpsc, watch};

const QUEUE_RECORDS: usize = 64;
pub struct Pump {
    pub records: mpsc::Receiver<Record>,
    pub failed: watch::Receiver<bool>,
    _failure: watch::Sender<bool>,
    stop: Arc<AtomicBool>,
    task: tokio::task::JoinHandle<()>,
}
impl Pump {
    pub fn start() -> Self {
        let (sender, records) = mpsc::channel(QUEUE_RECORDS);
        let (failure, failed) = watch::channel(false);
        let stop = Arc::new(AtomicBool::new(false));
        let stop_copy = Arc::clone(&stop);
        let failure_copy = failure.clone();
        let task = tokio::task::spawn_blocking(move || run(sender, failure_copy, stop_copy));
        Self { records, failed, _failure: failure, stop, task }
    }
    pub fn cancel(&self) {
        self.stop.store(true, Ordering::Release);
    }
    pub async fn finish(mut self) {
        self.cancel();
        if !matches!(tokio::time::timeout(Duration::from_secs(5), &mut self.task).await, Ok(Ok(())))
        {
            // Never report a completed stop while native handles remain alive.
            std::process::abort();
        }
    }
}
impl Drop for Pump {
    fn drop(&mut self) {
        self.cancel();
    }
}

fn run(sender: mpsc::Sender<Record>, failure: watch::Sender<bool>, stop: Arc<AtomicBool>) {
    use rokbattles_capture_adapters::{Receive, pcap::Pcap};
    let setup = (|| {
        let path = super::trust::library().map_err(|_error| ())?;
        let interfaces = super::interfaces::enumerate().map_err(|_error| ())?;
        // SAFETY: selected fixed OS library with protected ancestors; service
        // startup rejects loader overrides and runs from its protected install.
        let backend = unsafe { Pcap::load(&path) }.map_err(|_error| ())?;
        Ok::<_, ()>((backend, interfaces))
    })();
    let Ok((backend, interfaces)) = setup else {
        let _sent = sender.try_send(Record::Unavailable(UnavailableReason::NativeBackend));
        return;
    };
    let mut captures = Vec::new();
    for interface in &interfaces {
        let Ok(mut capture) = backend.open(&interface.name, &interface.addresses) else {
            let _sent = sender.try_send(Record::Unavailable(UnavailableReason::NativeBackend));
            return;
        };
        if capture.check_no_packet_loss().is_err() {
            let _sent = sender.try_send(Record::Unavailable(UnavailableReason::NativeBackend));
            return;
        }
        captures.push(capture);
    }
    if stop.load(Ordering::Acquire)
        || sender.try_send(Record::Started(rokbattles_capture_ipc::Backend::Pcap)).is_err()
    {
        return;
    }
    let mut last_interfaces = Instant::now();
    while !stop.load(Ordering::Acquire) {
        // New addresses/interfaces require newly scoped filters and a fresh Start.
        if last_interfaces.elapsed() >= Duration::from_secs(1) {
            if super::interfaces::enumerate().as_ref().ok() != Some(&interfaces) {
                break;
            }
            last_interfaces = Instant::now();
        }
        let mut active = false;
        for capture in &mut captures {
            if stop.load(Ordering::Acquire) {
                return;
            }
            if capture.check_no_packet_loss().is_err() {
                signal_loss(&failure, &stop);
                return;
            }
            let record = match capture.receive() {
                Ok(Receive::Packet(bytes)) => Some(Record::ServerPacket(bytes.into())),
                Ok(Receive::ClientControl(control)) => Some(Record::ClientControl(control)),
                Ok(Receive::Idle) => None,
                Ok(Receive::Discarded) => {
                    active = true;
                    None
                }
                Ok(Receive::End) | Err(_) => {
                    signal_loss(&failure, &stop);
                    return;
                }
            };
            // Wrap owned bytes before any loss/rejection branch so they are wiped.
            if capture.check_no_packet_loss().is_err() {
                signal_loss(&failure, &stop);
                return;
            }
            if let Some(record) = record {
                active = true;
                if sender.try_send(record).is_err() {
                    signal_loss(&failure, &stop);
                    return;
                }
            }
        }
        if !active {
            std::thread::sleep(Duration::from_millis(5));
        }
    }
    if !stop.load(Ordering::Acquire) {
        signal_loss(&failure, &stop);
    }
}
fn signal_loss(failure: &watch::Sender<bool>, stop: &AtomicBool) {
    let _sent = failure.send(true);
    stop.store(true, Ordering::Release);
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn source_loss_is_out_of_band_and_stops_capture_without_queue_space() {
        let (sender, mut receiver) = mpsc::channel(1);
        sender.try_send(Record::Keepalive).expect("full queue");
        let (failure, failed) = watch::channel(false);
        let stop = AtomicBool::new(false);
        signal_loss(&failure, &stop);
        assert!(*failed.borrow());
        assert!(stop.load(Ordering::Acquire));
        assert!(matches!(receiver.try_recv(), Ok(Record::Keepalive)));
    }
}
