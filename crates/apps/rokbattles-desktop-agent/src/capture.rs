//! Consent-gated bridge from OS-authenticated local capture to one ephemeral HTTP
//! stream. A lost byte retires the entire pipeline; reconnection starts with a
//! new Observer and requires fresh game connection evidence.

use std::{
    collections::VecDeque,
    future::Future,
    io,
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
    time::{Duration, Instant},
};

use rokbattles_capture_ipc::{Backend, ClientRequest, Record, read_record, write_request};
use rokbattles_capture_runtime::{
    lifecycle::{Event, Observer},
    packet,
};
use rokbattles_desktop_store::Store;
use tokio::{
    io::{AsyncRead, AsyncWrite},
    sync::mpsc,
    task::JoinHandle,
};
use zeroize::Zeroize;

use crate::{capture_http::CaptureUpload, maintenance_active};

const IPC_QUEUE: usize = 16;
const RECONNECT_DELAY: Duration = Duration::from_secs(5);
const POLL: Duration = Duration::from_millis(500);
const KEEPALIVE: Duration = Duration::from_secs(10);

// Stored/displayed codes, deliberately free of addresses, paths, IDs and errors.
const DISABLED: u8 = 0;
const CONNECTING: u8 = 1;
const WATCHING: u8 = 2;
const UPLOADING: u8 = 3;
const UNAVAILABLE: u8 = 4;
const LOST: u8 = 5;
const SETUP_REQUIRED: u8 = 6;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Failure {
    Revoked,
    Unavailable,
    SetupRequired,
    Lost,
}

trait Gate: Send + Sync {
    fn allowed(&self) -> impl Future<Output = bool> + Send;
}

impl Gate for Store {
    async fn allowed(&self) -> bool {
        self.settings().await.is_ok_and(|settings| {
            settings.enabled
                && !settings.paused
                && settings.capture_opt_in
                && !settings.maintenance_stop
                && maintenance_active().is_ok_and(|active| !active)
        })
    }
}

struct SessionGate<'a, G> {
    store: &'a G,
    failed: &'a AtomicBool,
}

impl<G: Gate> Gate for SessionGate<'_, G> {
    async fn allowed(&self) -> bool {
        !self.failed.load(Ordering::Acquire)
            && self.store.allowed().await
            && !self.failed.load(Ordering::Acquire)
    }
}

trait Upload: Send + 'static {
    fn event(&mut self, event: Event) -> Result<(), Failure>;
    fn ended(&self) -> bool;
    fn idle(&self) -> bool;
    fn finish(self) -> impl Future<Output = Result<(), Failure>> + Send;
}

impl Upload for CaptureUpload {
    fn event(&mut self, event: Event) -> Result<(), Failure> {
        self.event(event).map_err(|_error| Failure::Lost)
    }
    fn ended(&self) -> bool {
        self.ended()
    }
    fn idle(&self) -> bool {
        self.idle()
    }
    async fn finish(self) -> Result<(), Failure> {
        self.finish().await.map(|_| ()).map_err(|_error| Failure::Lost)
    }
}

/// Scrub events not consumed because consent, transport or a later event failed.
struct Events(VecDeque<Event>);
impl From<Vec<Event>> for Events {
    fn from(events: Vec<Event>) -> Self {
        Self(events.into())
    }
}
impl Drop for Events {
    fn drop(&mut self) {
        for event in &mut self.0 {
            if let Event::Data { bytes, .. } = event {
                bytes.zeroize();
            }
        }
    }
}

struct Pipeline<U, F> {
    observer: Observer,
    upload: Option<U>,
    factory: F,
    backend: Option<Backend>,
    keepalive: Duration,
}

impl<U: Upload, F: FnMut() -> Result<U, Failure>> Pipeline<U, F> {
    fn new(factory: F) -> Self {
        Self {
            observer: Observer::default(),
            upload: None,
            factory,
            backend: None,
            keepalive: Duration::ZERO,
        }
    }

    fn backend(&self) -> u8 {
        self.backend.map_or(0, |backend| backend as u8)
    }

    fn state(&self) -> u8 {
        if self.upload.is_some() {
            UPLOADING
        } else if self.backend.is_some() {
            WATCHING
        } else {
            CONNECTING
        }
    }

    async fn record(
        &mut self,
        record: Record,
        now: Duration,
        gate: &impl Gate,
    ) -> Result<(), Failure> {
        if !gate.allowed().await {
            return Err(Failure::Revoked);
        }
        if self.upload.as_ref().is_some_and(Upload::ended) {
            return Err(Failure::Lost);
        }
        let Some(backend) = self.backend else {
            return match record {
                Record::Started(backend) if allowed_backend(backend) => {
                    self.backend = Some(backend);
                    Ok(())
                }
                Record::Unavailable(_) => Err(Failure::Unavailable),
                _ => Err(Failure::Lost),
            };
        };
        let events = match record {
            Record::ServerPacket(bytes) => {
                let packet = packet::parse(&bytes).ok_or(Failure::Lost)?;
                self.observer.server(&packet, now)
            }
            Record::ClientControl(control) if permits_controls(backend) => {
                self.observer.client(control, now)
            }
            Record::SocketEstablished(evidence) if permits_sockets(backend) => {
                self.observer.socket_established(evidence, now)
            }
            Record::SocketRetired(evidence) if permits_sockets(backend) => {
                self.observer.socket_retired(evidence, now)
            }
            Record::Keepalive => Vec::new(),
            Record::Unavailable(_) => return Err(Failure::Unavailable),
            _ => return Err(Failure::Lost),
        };
        self.events(events, gate).await
    }

    async fn tick(&mut self, now: Duration, gate: &impl Gate) -> Result<(), Failure> {
        if !gate.allowed().await {
            return Err(Failure::Revoked);
        }
        if self.upload.as_ref().is_some_and(Upload::ended) {
            return Err(Failure::Lost);
        }
        let events = self.observer.expire(now);
        self.events(events, gate).await?;
        if self.upload.is_some() && now.saturating_sub(self.keepalive) >= KEEPALIVE {
            self.events(vec![Event::Keepalive], gate).await?;
            self.keepalive = now;
        }
        Ok(())
    }

    async fn events(&mut self, events: Vec<Event>, gate: &impl Gate) -> Result<(), Failure> {
        let mut events = Events::from(events);
        while !events.0.is_empty() {
            // The persisted gate is checked before each upload event, including
            // the first Open. No client control or socket evidence is serialized.
            if !gate.allowed().await {
                return Err(Failure::Revoked);
            }
            if self.upload.as_ref().is_some_and(Upload::ended) {
                return Err(Failure::Lost);
            }
            let is_open = matches!(events.0.front(), Some(Event::Open { .. }));
            if self.upload.is_none() {
                if !is_open {
                    return Err(Failure::Lost);
                }
                self.upload = Some((self.factory)()?);
            }
            let event = events.0.pop_front().ok_or(Failure::Lost)?;
            self.upload.as_mut().ok_or(Failure::Lost)?.event(event)?;
        }
        if self.upload.as_ref().is_some_and(Upload::idle) {
            self.upload.take().ok_or(Failure::Lost)?.finish().await?;
        }
        Ok(())
    }
}

fn allowed_backend(backend: Backend) -> bool {
    match backend {
        Backend::WinDivert => cfg!(all(windows, target_arch = "x86_64")),
        Backend::Pcap => cfg!(any(windows, target_os = "linux", target_os = "macos")),
    }
}
fn permits_controls(backend: Backend) -> bool {
    match backend {
        Backend::WinDivert => cfg!(all(windows, target_arch = "x86_64")),
        Backend::Pcap => cfg!(any(target_os = "linux", target_os = "macos")),
    }
}
fn permits_sockets(backend: Backend) -> bool {
    cfg!(windows) && backend == Backend::Pcap
}

struct Reader {
    records: mpsc::Receiver<Record>,
    failed: Arc<AtomicBool>,
    task: JoinHandle<()>,
}
impl Reader {
    fn start<R: AsyncRead + Unpin + Send + 'static>(mut input: R) -> Self {
        let (sender, records) = mpsc::channel(IPC_QUEUE);
        let failed = Arc::new(AtomicBool::new(false));
        let task_failed = Arc::clone(&failed);
        let task = tokio::spawn(async move {
            loop {
                // Only this task owns the read future. Timeouts terminate the
                // connection, never restart parsing in the middle of a frame.
                let record =
                    tokio::time::timeout(Duration::from_secs(5), read_record(&mut input)).await;
                let Ok(Ok(record)) = record else {
                    break;
                };
                if sender.try_send(record).is_err() {
                    break;
                }
            }
            task_failed.store(true, Ordering::Release);
        });
        Self { records, failed, task }
    }
    fn failed(&self) -> bool {
        self.failed.load(Ordering::Acquire)
    }
}
impl Drop for Reader {
    fn drop(&mut self) {
        self.task.abort();
    }
}

pub(crate) struct Worker {
    task: Option<JoinHandle<()>>,
}
impl Worker {
    pub fn spawn(store: Arc<Store>) -> Self {
        let task = tokio::spawn(async move {
            loop {
                if run(&store).await.is_err() {
                    let _status = store.capture_status(UNAVAILABLE, 0).await;
                }
                tokio::time::sleep(RECONNECT_DELAY).await;
            }
        });
        Self { task: Some(task) }
    }
    pub async fn stop(mut self) {
        if let Some(task) = self.task.take() {
            task.abort();
            let _result = task.await;
        }
    }
}
impl Drop for Worker {
    fn drop(&mut self) {
        if let Some(task) = &self.task {
            task.abort();
        }
    }
}

async fn run(store: &Store) -> anyhow::Result<()> {
    loop {
        if !store.allowed().await {
            store.capture_status(DISABLED, 0).await?;
            while !store.allowed().await {
                tokio::time::sleep(POLL).await;
            }
        }
        store.capture_status(CONNECTING, 0).await?;
        let result = match tokio::time::timeout(Duration::from_secs(5), connect()).await {
            Ok(Ok(stream)) => session(stream, store).await,
            Ok(Err(error)) if error.kind() == io::ErrorKind::NotFound => {
                Err(Failure::SetupRequired)
            }
            _ => Err(Failure::Unavailable),
        };
        store
            .capture_status(
                match result {
                    Err(Failure::Revoked) => DISABLED,
                    Err(Failure::Unavailable) => UNAVAILABLE,
                    Err(Failure::SetupRequired) => SETUP_REQUIRED,
                    _ => LOST,
                },
                0,
            )
            .await?;
        tokio::time::sleep(RECONNECT_DELAY).await;
    }
}

async fn connect() -> io::Result<impl AsyncRead + AsyncWrite + Unpin + Send> {
    #[cfg(windows)]
    {
        rokbattles_capture_ipc::windows::connect_current_user()
    }
    #[cfg(any(target_os = "linux", target_os = "macos"))]
    {
        connect_protected(
            std::env::current_exe()?,
            crate::installed_capture_agent()?,
            rokbattles_capture_ipc::unix::connect_current_user,
        )
        .await
    }
}

#[cfg(any(target_os = "linux", target_os = "macos"))]
async fn connect_protected<T, F: Future<Output = io::Result<T>>>(
    current: std::path::PathBuf,
    protected: Option<std::path::PathBuf>,
    connector: impl FnOnce() -> F,
) -> io::Result<T> {
    if protected.as_ref() != Some(&current) {
        // Do not even invoke the connector for a bundled mailcache image.
        return Err(io::Error::new(
            io::ErrorKind::NotFound,
            "protected capture companion required",
        ));
    }
    connector().await
}

async fn session<S: AsyncRead + AsyncWrite + Unpin + Send + 'static>(
    mut stream: S,
    store: &Store,
) -> Result<(), Failure> {
    if !store.allowed().await {
        return Err(Failure::Revoked);
    }
    write_request(&mut stream, ClientRequest::Start).await.map_err(|_error| Failure::Lost)?;
    let (input, mut output) = tokio::io::split(stream);
    let mut reader = Reader::start(input);
    let failure = Arc::clone(&reader.failed);
    let gate = SessionGate { store, failed: &failure };
    let mut pipeline = Pipeline::new(|| CaptureUpload::start().map_err(|_error| Failure::Lost));
    let start = Instant::now();
    let mut timer = tokio::time::interval(POLL);
    timer.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
    let mut status = (CONNECTING, 0);
    let result = async {
        loop {
            if reader.failed() {
                return Err(Failure::Lost);
            }
            tokio::select! {
                record = reader.records.recv() => {
                    if reader.failed() { return Err(Failure::Lost); }
                    pipeline.record(record.ok_or(Failure::Lost)?, start.elapsed(), &gate).await?;
                }
                _ = timer.tick() => pipeline.tick(start.elapsed(), &gate).await?,
            }
            let next = (pipeline.state(), pipeline.backend());
            if status != next {
                store.capture_status(next.0, next.1).await.map_err(|_error| Failure::Lost)?;
                status = next;
            }
        }
    }
    .await;
    // Drop all HTTP/observer/queued-packet state before requesting helper stop.
    drop(pipeline);
    let result = if reader.failed() { Err(Failure::Lost) } else { result };
    drop(reader);
    let _stop = write_request(&mut output, ClientRequest::Stop).await;
    result
}

#[cfg(test)]
mod tests;
