use std::{
    collections::BTreeMap,
    path::PathBuf,
    time::{Duration, Instant},
};

use rokbattles_desktop_store::{PendingFile, Root, Store};
use zeroize::{Zeroize, Zeroizing};

use crate::{
    mailcache::{MailRoot, Observation, Scan},
    now_ms,
    upload::{self, Outcome},
};

struct RootScan {
    root: MailRoot,
    cursor: Option<Scan>,
    refresh_after: Instant,
}

pub struct Agent {
    store: Store,
    client: reqwest::Client,
    scans: BTreeMap<i64, RootScan>,
    cursor: usize,
    heartbeat: Instant,
}

impl Agent {
    pub fn new(store: Store) -> anyhow::Result<Self> {
        Ok(Self {
            store,
            client: upload::client()?,
            scans: BTreeMap::new(),
            cursor: 0,
            heartbeat: Instant::now()
                .checked_sub(Duration::from_secs(2))
                .unwrap_or_else(Instant::now),
        })
    }

    /// One bounded iteration, independently of any Tauri window/app handle.
    pub async fn tick(&mut self) -> anyhow::Result<bool> {
        let settings = self.store.settings().await?;
        if !settings.enabled {
            self.store.heartbeat(now_ms(), false, settings.paused, 0, 0).await?;
            return Ok(false);
        }
        if settings.paused {
            self.store.heartbeat(now_ms(), true, true, 0, 0).await?;
            return Ok(true);
        }
        if settings.reset_epoch != 0 {
            self.store.clear_history().await?;
            self.store.acknowledge_reprocess(settings.reset_epoch).await?;
            self.scans.clear();
        }
        let roots = self.store.roots().await?;
        let now = now_ms();
        let mut scans = std::mem::take(&mut self.scans);
        let cursor = self.cursor;
        let (returned, observations, scan_error) = tokio::task::spawn_blocking(move || {
            scans.retain(|id, scan| {
                roots.iter().any(|root| root.id == *id && root.path == scan.root.path())
            });
            let mut failed = false;
            for root in roots {
                if let std::collections::btree_map::Entry::Vacant(entry) = scans.entry(root.id) {
                    match MailRoot::open(&root.path) {
                        Ok(root) => {
                            entry.insert(RootScan {
                                root,
                                cursor: None,
                                refresh_after: Instant::now(),
                            });
                        }
                        Err(_) => failed = true,
                    }
                }
            }
            let mut output = Vec::new();
            if !scans.is_empty() {
                let index = cursor % scans.len();
                if let Some((id, scan)) = scans.iter_mut().nth(index) {
                    match scan_batch(scan, now) {
                        Ok(items) => output = items.into_iter().map(|item| (*id, item)).collect(),
                        Err(_) => failed = true,
                    }
                }
            }
            (scans, output, failed)
        })
        .await?;
        self.scans = returned;
        self.cursor = self.cursor.wrapping_add(1);
        for (root, item) in observations {
            self.store.observe(root, &item.path, item.sig).await?;
        }
        let mut event = if scan_error { 1 } else { 0 };
        if let Some(item) = self.store.next_pending(now).await? {
            event = self.process(item).await?;
        }
        // Capture backend integration follows separately; status truthfully
        // reports mailcache even when opt-in is set but no helper is connected.
        if self.heartbeat.elapsed() >= Duration::from_secs(1) || event != 0 {
            self.store.heartbeat(now_ms(), true, false, 3, event).await?;
            self.heartbeat = Instant::now();
        }
        Ok(true)
    }

    async fn process(&self, item: PendingFile) -> anyhow::Result<u8> {
        let root = self.store.roots().await?.into_iter().find(|root| root.id == item.root_id);
        let Some(root) = root else {
            return Ok(0);
        };
        let path = item.path.clone();
        let sig = item.sig;
        let prepared = tokio::task::spawn_blocking(move || prepare(&root, path, sig)).await?;
        let bytes = match prepared {
            Prepared::Ready(bytes) => bytes,
            Prepared::Missing => {
                self.store.forget_missing(&item).await?;
                return Ok(0);
            }
            Prepared::Unsupported => {
                self.store.finish(&item, false).await?;
                return Ok(2);
            }
            Prepared::Retry => {
                self.store.retry(&item, now_ms().saturating_add(5000)).await?;
                return Ok(1);
            }
        };
        let Some(name) = item.path.file_name().and_then(|name| name.to_str()) else {
            return Ok(1);
        };
        // Desired settings are checked again immediately before transmission.
        let settings = self.store.settings().await?;
        if settings.paused || !settings.enabled {
            return Ok(0);
        }
        match upload::send(&self.client, name, bytes, item.attempts).await {
            Outcome::Stored => {
                self.store.finish(&item, true).await?;
                Ok(3)
            }
            Outcome::Rejected => {
                self.store.finish(&item, false).await?;
                Ok(2)
            }
            Outcome::Retry(delay) => {
                self.store.retry(&item, now_ms().saturating_add(delay.as_millis() as u64)).await?;
                Ok(4)
            }
        }
    }

    pub async fn close(self) {
        let _status = self.store.heartbeat(now_ms(), false, false, 0, 0).await;
        self.store.close().await;
    }
}

fn scan_batch(scan: &mut RootScan, now_ms: u64) -> anyhow::Result<Vec<Observation>> {
    if scan.cursor.is_none() {
        if Instant::now() < scan.refresh_after {
            return Ok(Vec::new());
        }
        // Reopen each full pass so replaced/deleted roots do not stay pinned.
        scan.root = MailRoot::open(scan.root.path())?;
        scan.cursor = Some(scan.root.scan()?);
    }
    let Some(cursor) = scan.cursor.as_mut() else {
        return Ok(Vec::new());
    };
    let result = scan.root.scan_next(cursor, now_ms);
    if !result.as_ref().is_ok_and(|(_, done)| !*done) {
        scan.cursor = None;
        scan.refresh_after = Instant::now() + Duration::from_secs(5);
    }
    result.map(|(items, _)| items)
}

enum Prepared {
    Ready(Zeroizing<Vec<u8>>),
    Unsupported,
    Missing,
    Retry,
}

fn prepare(root: &Root, path: PathBuf, sig: rokbattles_desktop_store::FileSig) -> Prepared {
    let Ok(root) = MailRoot::open(&root.path) else {
        return Prepared::Retry;
    };
    let bytes = match root.read(&path, sig) {
        Ok(bytes) => bytes,
        Err(error)
            if error
                .downcast_ref::<std::io::Error>()
                .is_some_and(|error| error.kind() == std::io::ErrorKind::NotFound) =>
        {
            return Prepared::Missing;
        }
        Err(_) => return Prepared::Retry,
    };
    let Ok(decoded) = rokbattles_mail_codec::decode_bounded(&bytes, 500_000) else {
        return Prepared::Unsupported;
    };
    let decoded = Decoded(decoded);
    if rokbattles_mail_registry::detect_mail_type(&decoded.0).is_none() {
        return Prepared::Unsupported;
    }
    Prepared::Ready(bytes)
}

// Best-effort erasure of owned decoded values after classification. This does
// not promise removal of allocator copies or operating-system memory snapshots.
struct Decoded(serde_json::Value);

impl Drop for Decoded {
    fn drop(&mut self) {
        wipe_value(&mut self.0);
    }
}

fn wipe_value(value: &mut serde_json::Value) {
    match std::mem::take(value) {
        serde_json::Value::String(mut value) => value.zeroize(),
        serde_json::Value::Array(mut values) => {
            for value in &mut values {
                wipe_value(value);
            }
        }
        serde_json::Value::Object(values) => {
            for (mut key, mut value) in values {
                key.zeroize();
                wipe_value(&mut value);
            }
        }
        _ => {}
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn decoded_value_cleanup_replaces_root_and_nested_values() {
        let mut value =
            serde_json::json!({"private key": ["private value", {"nested": "content"}]});
        wipe_value(&mut value);
        assert!(value.is_null());
    }

    #[tokio::test]
    async fn agent_without_ui_obeys_pause_reset_and_stop_without_network() {
        let temp = tempfile::tempdir().expect("tempdir");
        let directory = temp.path().canonicalize().expect("canonical").join("state");
        let store = Store::open(&directory).await.expect("store");
        store.set_paused(true).await.expect("pause");
        let mut agent = Agent::new(store).expect("agent");
        assert!(agent.tick().await.expect("paused tick"));
        assert!(agent.store.status().await.expect("status").paused);
        agent.store.request_reprocess().await.expect("reset");
        agent.store.set_paused(false).await.expect("resume");
        assert!(agent.tick().await.expect("running tick"));
        assert_eq!(agent.store.settings().await.expect("settings").reset_epoch, 0);
        agent.store.set_enabled(false).await.expect("stop");
        assert!(!agent.tick().await.expect("stop tick"));
        agent.close().await;
    }
}
