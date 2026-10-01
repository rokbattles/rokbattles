//! Default-off anonymous desktop capture intake. No authentication is implied.
//! Only reconstructed mails reach MongoDB; stream/cipher state is request-local.
//! Enabling requires a TLS edge with request buffering/body logging disabled and
//! appropriate connection/rate limits. Peer limits use the actual socket peer;
//! forwarding headers are deliberately never trusted.

use std::{
    collections::BTreeMap,
    future::Future,
    net::{IpAddr, SocketAddr},
    sync::{Arc, Mutex},
    time::{Duration, Instant},
};

use axum::{
    Json,
    body::Body,
    extract::{ConnectInfo, State},
    http::{HeaderMap, StatusCode},
    response::{IntoResponse, Response},
};
use futures::{Stream, StreamExt};
use mongodb::bson::{DateTime, Document};
use rokbattles_capture_runtime::wire::{self, MAX_CHUNK};
use rokbattles_gateway_protocol::{
    RuntimeArtifact,
    capture_session::{CapturedMail, Session},
};
use rokbattles_mail_reconstructor::{ReconstructionContext, ReconstructionError};
use serde::Serialize;
use tokio::sync::{OwnedSemaphorePermit, Semaphore};
use zeroize::Zeroizing;

use crate::{
    raw_mail::{self, RawMailDocumentInput},
    state::AppState,
};

// Size admission for worst-case session memory, not average traffic. Each live
// session permits 64 MiB partial frames. At most two hold decoded/reconstructed
// output concurrently. Reserve at least 1 GiB for this feature plus normal ingress.
const MAX_SESSIONS: usize = 4;
const MAX_PER_PEER: usize = 2;
const MAX_PEERS: usize = 4096;
const BODY_IDLE: Duration = Duration::from_secs(30);
const EMPTY_IDLE: Duration = Duration::from_secs(30);
const STORE_TIMEOUT: Duration = Duration::from_secs(5);
const RETRY_DELAY: Duration = Duration::from_millis(250);
const MAX_OUTPUT_BYTES: usize = 25 * 1024 * 1024;
const MAX_BSON_BYTES: usize = 15 * 1024 * 1024;

pub struct Intake {
    artifact: RuntimeArtifact,
    sessions: Arc<Semaphore>,
    heavy: Arc<Semaphore>,
    peers: Mutex<BTreeMap<IpAddr, Peer>>,
}

struct Peer {
    active: usize,
    last: Instant,
}

impl Intake {
    pub fn new(artifact: RuntimeArtifact) -> Self {
        Self {
            artifact,
            sessions: Arc::new(Semaphore::new(MAX_SESSIONS)),
            heavy: Arc::new(Semaphore::new(2)),
            peers: Mutex::new(BTreeMap::new()),
        }
    }

    fn admit(self: &Arc<Self>, peer: IpAddr) -> Result<Admission, StatusCode> {
        let permit = self
            .sessions
            .clone()
            .try_acquire_owned()
            .map_err(|_error| StatusCode::TOO_MANY_REQUESTS)?;
        let mut peers = self.peers.lock().map_err(|_error| StatusCode::SERVICE_UNAVAILABLE)?;
        peers.retain(|_, peer| peer.active != 0 || peer.last.elapsed() < Duration::from_secs(60));

        if peers.get(&peer).is_some_and(|entry| entry.active >= MAX_PER_PEER)
            || (!peers.contains_key(&peer) && peers.len() >= MAX_PEERS)
        {
            return Err(StatusCode::TOO_MANY_REQUESTS);
        }
        let entry = peers.entry(peer).or_insert(Peer { active: 0, last: Instant::now() });
        entry.active += 1;
        entry.last = Instant::now();
        Ok(Admission { intake: self.clone(), peer, _permit: permit })
    }
}

struct Admission {
    intake: Arc<Intake>,
    peer: IpAddr,
    _permit: OwnedSemaphorePermit,
}

impl Drop for Admission {
    fn drop(&mut self) {
        if let Ok(mut peers) = self.intake.peers.lock()
            && let Some(peer) = peers.get_mut(&self.peer)
        {
            peer.active = peer.active.saturating_sub(1);
            peer.last = Instant::now();
        }
    }
}

#[derive(Default, Serialize)]
struct Report {
    stored: u64,
    skipped: u64,
    rejected: u64,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Outcome {
    Stored,
    Skipped,
    Rejected,
}

pub(crate) fn register<S: Clone + Send + Sync + 'static>(
    router: axum::Router<S>,
    enabled: bool,
    handler: axum::routing::MethodRouter<S>,
) -> axum::Router<S> {
    if enabled { router.route("/v3/desktop/capture", handler) } else { router }
}

/// This handler is not registered at all while DESKTOP_CAPTURE_ENABLED=false.
pub async fn upload(
    State(state): State<Arc<AppState>>,
    ConnectInfo(peer): ConnectInfo<SocketAddr>,
    headers: HeaderMap,
    body: Body,
) -> Response {
    let Some(intake) = state.capture.as_ref() else {
        return StatusCode::NOT_FOUND.into_response();
    };
    if !valid_headers(&headers) {
        return StatusCode::UNSUPPORTED_MEDIA_TYPE.into_response();
    }
    let _admission = match intake.admit(peer.ip()) {
        Ok(admission) => admission,
        Err(status) => return status.into_response(),
    };
    let mut reconstruct_credit = Credits::new(64 * 1024 * 1024, 2 * 1024 * 1024);
    let mut session = Session::new(&intake.artifact, Duration::ZERO);
    let result = consume(body.into_data_stream(), &mut session, &intake.heavy, |mail| {
        reconstruct_credit.refill();
        let mut remaining = reconstruct_credit.available;
        let before = remaining;
        let prepared = tokio::task::block_in_place(|| prepare(&state, mail, &mut remaining));
        reconstruct_credit.available = remaining.min(before);

        async {
            match prepared {
                Ok(Some(mail)) => retry_store(
                    || async {
                        state
                            .storage
                            .insert_capture_once(&mail.id, mail.document.clone())
                            .await
                            .map_err(|_error| ())
                    },
                    STORE_TIMEOUT,
                    RETRY_DELAY,
                )
                .await
                .map(|inserted| if inserted { Outcome::Stored } else { Outcome::Skipped }),
                Ok(None) => Ok(Outcome::Rejected),
                Err(status) => Err(status),
            }
        }
    })
    .await;

    // A non-success can follow successful stores. It is not transactional and
    // must make the desktop discard active cipher state, not replay ciphertext.
    match result {
        Ok(report) => Json(report).into_response(),
        Err(status) => status.into_response(),
    }
}

fn valid_headers(headers: &HeaderMap) -> bool {
    let media = headers.get("content-type").and_then(|value| value.to_str().ok());
    let identity = match headers.get("content-encoding") {
        None => true,
        Some(value) => value.to_str().is_ok_and(|value| value.eq_ignore_ascii_case("identity")),
    };
    media.is_some_and(|value| value.eq_ignore_ascii_case("application/octet-stream"))
        && identity
        && headers.get_all("content-type").iter().count() == 1
        && headers.get_all("content-encoding").iter().count() <= 1
        && !headers.contains_key("authorization")
}

async fn consume<S, E, F, Fut>(
    mut body: S,
    session: &mut Session<'_>,
    heavy: &Arc<Semaphore>,
    mut store: F,
) -> Result<Report, StatusCode>
where
    S: Stream<Item = Result<bytes::Bytes, E>> + Unpin,
    F: FnMut(CapturedMail) -> Fut,
    Fut: Future<Output = Result<Outcome, StatusCode>>,
{
    let started = Instant::now();
    let mut empty_since = Some(Instant::now());
    let mut wire = wire::Decoder::default();
    let mut bytes_credit = Credits::new(32 * 1024 * 1024, 1024 * 1024);
    let mut event_credit = Credits::new(4096, 64);
    let mut report = Report::default();

    loop {
        session.expire(started.elapsed()).map_err(|_error| StatusCode::UNPROCESSABLE_ENTITY)?;
        if session.flow_count() == 0 {
            let empty = empty_since.get_or_insert_with(Instant::now);
            if empty.elapsed() >= EMPTY_IDLE {
                return Err(StatusCode::REQUEST_TIMEOUT);
            }
        } else {
            empty_since = None;
        }

        let next = tokio::time::timeout(BODY_IDLE, body.next())
            .await
            .map_err(|_error| StatusCode::REQUEST_TIMEOUT)?;
        let Some(chunk) = next else {
            break;
        };
        let chunk = chunk.map_err(|_error| StatusCode::BAD_REQUEST)?;
        bytes_credit.take(chunk.len())?;

        for fragment in chunk.chunks(MAX_CHUNK) {
            let mut incoming = Vec::new();
            wire.push(fragment, |event| {
                event_credit.take(1).map_err(|_status| wire::Error::TooLarge)?;
                incoming.push(event);
                Ok(())
            })
            .map_err(|_error| StatusCode::BAD_REQUEST)?;

            for event in incoming {
                let _heavy = tokio::time::timeout(BODY_IDLE, heavy.clone().acquire_owned())
                    .await
                    .map_err(|_error| StatusCode::SERVICE_UNAVAILABLE)?
                    .map_err(|_error| StatusCode::SERVICE_UNAVAILABLE)?;
                let mails =
                    tokio::task::block_in_place(|| session.accept(event, started.elapsed()))
                        .map_err(|_error| StatusCode::UNPROCESSABLE_ENTITY)?;

                // No queue/spawn: keep only bounded decoded output and stop polling
                // the body until every returned mail has a final storage outcome.
                for mail in mails {
                    match store(mail).await? {
                        Outcome::Stored => report.stored = report.stored.saturating_add(1),
                        Outcome::Skipped => report.skipped = report.skipped.saturating_add(1),
                        Outcome::Rejected => report.rejected = report.rejected.saturating_add(1),
                    }
                }
            }
        }
    }

    wire.finish().map_err(|_error| StatusCode::BAD_REQUEST)?;
    Ok(report)
}

struct PreparedMail {
    id: String,
    document: Document,
}

fn prepare(
    state: &AppState,
    mail: CapturedMail,
    remaining: &mut usize,
) -> Result<Option<PreparedMail>, StatusCode> {
    let context = ReconstructionContext {
        player_id: mail.context.player_id,
        server_id: mail.context.server_id,
    };
    let reconstructed =
        match state.mail_reconstructor.reconstruct_with_budget(&mail.entry, context, remaining) {
            Ok(mail) => mail,
            Err(ReconstructionError::BudgetExceeded) => return Err(StatusCode::TOO_MANY_REQUESTS),
            Err(_) => return Ok(None),
        };
    let bytes = Zeroizing::new(reconstructed.bytes);
    if bytes.len() > MAX_OUTPUT_BYTES {
        return Err(StatusCode::PAYLOAD_TOO_LARGE);
    }
    let decoded =
        rokbattles_mail_codec::decode(&bytes).map_err(|_error| StatusCode::UNPROCESSABLE_ENTITY)?;
    let metadata = raw_mail::extract_raw_mail_metadata(&decoded)
        .map_err(|_error| StatusCode::UNPROCESSABLE_ENTITY)?;
    if metadata.id != reconstructed.id {
        return Err(StatusCode::UNPROCESSABLE_ENTITY);
    }
    let checksum = raw_mail::sha256_hex(&bytes);
    let document = raw_mail::build_raw_mail_doc(RawMailDocumentInput {
        original_bytes: &bytes,
        user_agent: "ROKBattles/DesktopCapture",
        checksum: &checksum,
        mail: &metadata,
        status: "pending",
        now: DateTime::now(),
        zstd_level: 6,
    })
    .map_err(|_error| StatusCode::SERVICE_UNAVAILABLE)?;
    if !fits_bson_limit(&document)? {
        return Ok(None);
    }
    Ok(Some(PreparedMail { id: metadata.id, document }))
}

fn fits_bson_limit(document: &Document) -> Result<bool, StatusCode> {
    let encoded = Zeroizing::new(
        mongodb::bson::to_vec(document).map_err(|_error| StatusCode::UNPROCESSABLE_ENTITY)?,
    );
    Ok(encoded.len() <= MAX_BSON_BYTES)
}

async fn retry_store<F, Fut>(
    mut store: F,
    timeout: Duration,
    delay: Duration,
) -> Result<bool, StatusCode>
where
    F: FnMut() -> Fut,
    Fut: Future<Output = Result<bool, ()>>,
{
    for attempt in 0..3u32 {
        match tokio::time::timeout(timeout, store()).await {
            Ok(Ok(inserted)) => return Ok(inserted),
            _ if attempt < 2 => tokio::time::sleep(delay.saturating_mul(attempt + 1)).await,
            _ => return Err(StatusCode::SERVICE_UNAVAILABLE),
        }
    }
    Err(StatusCode::SERVICE_UNAVAILABLE)
}

struct Credits {
    available: usize,
    capacity: usize,
    per_second: usize,
    last: Instant,
}

impl Credits {
    fn new(capacity: usize, per_second: usize) -> Self {
        Self { available: capacity, capacity, per_second, last: Instant::now() }
    }

    fn refill(&mut self) {
        let milliseconds = self.last.elapsed().as_millis();
        if milliseconds == 0 {
            return;
        }
        let added = milliseconds.saturating_mul(self.per_second as u128) / 1000;
        self.available = self
            .available
            .saturating_add(usize::try_from(added).unwrap_or(usize::MAX))
            .min(self.capacity);
        self.last += Duration::from_millis(u64::try_from(milliseconds).unwrap_or(u64::MAX));
    }

    fn take(&mut self, amount: usize) -> Result<(), StatusCode> {
        self.refill();
        self.available = self.available.checked_sub(amount).ok_or(StatusCode::TOO_MANY_REQUESTS)?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rokbattles_capture_runtime::lifecycle::Event;
    use rokbattles_gateway_protocol::stream::test_server_frames;
    use std::sync::atomic::{AtomicUsize, Ordering};

    #[test]
    fn global_and_peer_admission_release_on_disconnect() {
        let intake = Arc::new(Intake::new(RuntimeArtifact::test_fixture()));
        let peer = "192.0.2.1".parse().expect("peer");
        let first = intake.admit(peer).expect("first");
        let second = intake.admit(peer).expect("second");
        assert_eq!(intake.admit(peer).err(), Some(StatusCode::TOO_MANY_REQUESTS));
        drop(first);
        intake.admit(peer).expect("released slot");
        drop(second);
        assert_eq!(intake.sessions.available_permits(), MAX_SESSIONS);
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn stores_mail_before_eof_and_applies_backpressure() {
        let artifact = RuntimeArtifact::test_fixture();
        let mut session = Session::new(&artifact, Duration::ZERO);
        let mut events = vec![Event::Open { id: 1, server_port: 3101 }];
        let mut offset = 0;
        for frame in test_server_frames(1) {
            let length = frame.len();
            events.push(Event::Data { id: 1, offset, bytes: frame });
            offset += length as u64;
        }
        let chunks: Vec<_> = events
            .iter()
            .flat_map(|event| wire::encode(event).expect("encode"))
            .map(|bytes| Ok::<_, ()>(bytes::Bytes::from(bytes)))
            .collect();
        let seen = AtomicUsize::new(0);
        let body = futures::stream::iter(chunks).chain(futures::stream::poll_fn(|_context| {
            // The consumer must have awaited storage before it asks for EOF.
            assert_eq!(seen.load(Ordering::SeqCst), 1);
            std::task::Poll::Ready(None)
        }));
        let report = consume(body, &mut session, &Arc::new(Semaphore::new(1)), |_mail| async {
            seen.fetch_add(1, Ordering::SeqCst);
            Ok(Outcome::Stored)
        })
        .await
        .expect("consume");
        assert_eq!(report.stored, 1);
    }

    #[tokio::test]
    async fn transient_store_failure_retries_but_exhaustion_is_not_success() {
        let attempts = AtomicUsize::new(0);
        let result = retry_store(
            || async {
                if attempts.fetch_add(1, Ordering::SeqCst) < 2 { Err(()) } else { Ok(true) }
            },
            Duration::from_secs(1),
            Duration::ZERO,
        )
        .await
        .expect("third attempt");
        assert!(result);
        assert_eq!(attempts.load(Ordering::SeqCst), 3);
        assert_eq!(
            retry_store(|| async { Err(()) }, Duration::from_secs(1), Duration::ZERO)
                .await
                .expect_err("unavailable"),
            StatusCode::SERVICE_UNAVAILABLE
        );
    }

    #[test]
    fn rejects_encoded_bodies_and_does_not_accept_relay_tokens() {
        let mut headers = HeaderMap::new();
        headers.insert("content-type", "application/octet-stream".parse().expect("header"));
        assert!(valid_headers(&headers));
        headers.insert("content-encoding", "gzip".parse().expect("header"));
        assert!(!valid_headers(&headers));
        headers.remove("content-encoding");
        headers.insert("authorization", "Bearer not-accepted".parse().expect("header"));
        assert!(!valid_headers(&headers));
    }

    #[test]
    fn bson_storage_cap_is_checked_before_retry() {
        use mongodb::bson::{Binary, doc, spec::BinarySubtype};
        let near = doc! { "binary": Binary { subtype: BinarySubtype::Generic, bytes: vec![42; MAX_BSON_BYTES - 1024] } };
        assert!(fits_bson_limit(&near).expect("serialize"));
        let over = doc! { "binary": Binary { subtype: BinarySubtype::Generic, bytes: vec![42; MAX_BSON_BYTES] } };
        assert!(!fits_bson_limit(&over).expect("serialize"));
    }

    #[tokio::test]
    async fn route_is_absent_until_explicitly_registered() {
        use tower::ServiceExt;
        for (enabled, expected) in [(false, StatusCode::NOT_FOUND), (true, StatusCode::NO_CONTENT)]
        {
            let router = register(
                axum::Router::new(),
                enabled,
                axum::routing::post(|| async { StatusCode::NO_CONTENT }),
            );
            let request = axum::http::Request::post("/v3/desktop/capture")
                .body(Body::empty())
                .expect("request");
            assert_eq!(router.oneshot(request).await.expect("router").status(), expected);
        }
    }

    #[tokio::test]
    async fn cancellation_releases_global_and_peer_admission() {
        let intake = Arc::new(Intake::new(RuntimeArtifact::test_fixture()));
        let peer = "192.0.2.1".parse().expect("peer");
        let admission = intake.admit(peer).expect("admit");
        let task = tokio::spawn(async move {
            let _admission = admission;
            std::future::pending::<()>().await;
        });
        tokio::task::yield_now().await;
        task.abort();
        assert!(task.await.expect_err("cancelled").is_cancelled());
        assert_eq!(intake.sessions.available_permits(), MAX_SESSIONS);
        assert_eq!(intake.peers.lock().expect("peers").get(&peer).expect("peer").active, 0);
    }

    #[tokio::test(start_paused = true)]
    async fn idle_body_and_truncated_eof_fail_closed() {
        let artifact = RuntimeArtifact::test_fixture();
        let mut session = Session::new(&artifact, Duration::ZERO);
        let heavy = Arc::new(Semaphore::new(1));
        let idle = futures::stream::pending::<Result<bytes::Bytes, ()>>();
        let result =
            consume(idle, &mut session, &heavy, |_mail| async { Ok(Outcome::Stored) }).await;
        assert_eq!(result.err(), Some(StatusCode::REQUEST_TIMEOUT));
        let partial = futures::stream::iter([Ok::<_, ()>(bytes::Bytes::from_static(b"RBC"))]);
        let result =
            consume(partial, &mut session, &heavy, |_mail| async { Ok(Outcome::Stored) }).await;
        assert_eq!(result.err(), Some(StatusCode::BAD_REQUEST));
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn later_bad_offset_does_not_undo_stored_mail_or_allow_cipher_replay() {
        let artifact = RuntimeArtifact::test_fixture();
        let mut session = Session::new(&artifact, Duration::ZERO);
        let mut events = vec![Event::Open { id: 1, server_port: 5222 }];
        let mut offset = 0;
        for frame in test_server_frames(1) {
            let length = frame.len();
            events.push(Event::Data { id: 1, offset, bytes: frame });
            offset += length as u64;
        }
        events.push(Event::Data { id: 1, offset: 0, bytes: vec![0] });
        let chunks: Vec<_> = events
            .iter()
            .flat_map(|event| wire::encode(event).expect("frame"))
            .map(|bytes| Ok::<_, ()>(bytes::Bytes::from(bytes)))
            .collect();
        let stored = AtomicUsize::new(0);
        let result = consume(
            futures::stream::iter(chunks),
            &mut session,
            &Arc::new(Semaphore::new(1)),
            |_mail| async {
                stored.fetch_add(1, Ordering::SeqCst);
                Ok(Outcome::Stored)
            },
        )
        .await;
        assert_eq!(result.err(), Some(StatusCode::UNPROCESSABLE_ENTITY));
        assert_eq!(stored.load(Ordering::SeqCst), 1);
        assert_eq!(session.flow_count(), 0);
        session
            .accept(Event::Open { id: 2, server_port: 5222 }, Duration::ZERO)
            .expect_err("poisoned session cannot replay");
    }

    #[test]
    fn byte_and_event_credits_reject_exhaustion_without_underflow() {
        let mut credit = Credits::new(2, 0);
        credit.take(2).expect("exact credit");
        assert_eq!(credit.take(1), Err(StatusCode::TOO_MANY_REQUESTS));
        assert_eq!(credit.available, 0);
    }
}
