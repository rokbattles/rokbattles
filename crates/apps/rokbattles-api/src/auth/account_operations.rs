//! Coordinates account creation and deletion within one API process.
//!
//! Operations that create users, sessions, or governor binds hold the same lock
//! as account deletion. This is process-local coordination; multiple API instances
//! would require a shared locking protocol instead.

use std::sync::{Arc, Mutex, PoisonError};

use mongodb::bson::DateTime;
use rustc_hash::FxHashMap;
use tokio::sync::{Mutex as AsyncMutex, OwnedMutexGuard};

// OAuth state expires after ten minutes. Older deletion markers are unnecessary
// because a sign-in begun before that deletion can no longer complete.
const LOGIN_WINDOW_MILLIS: i64 = 10 * 60 * 1000;

#[derive(Default)]
struct Registry {
    locks: FxHashMap<String, Arc<AsyncMutex<()>>>,
    deleted_at: FxHashMap<String, DateTime>,
}

/// Shared account locks and recent deletion times for this API instance.
#[derive(Clone, Default)]
pub(crate) struct AccountOperations(Arc<Mutex<Registry>>);

impl AccountOperations {
    /// Waits for earlier account writes and holds off later writes until dropped.
    pub(crate) async fn lock(&self, user_id: &str) -> AccountOperation {
        let lock = {
            let mut registry = self.0.lock().unwrap_or_else(PoisonError::into_inner);
            let cutoff = DateTime::now().timestamp_millis() - LOGIN_WINDOW_MILLIS;

            registry.locks.retain(|_, lock| Arc::strong_count(lock) > 1);
            registry.deleted_at.retain(|_, time| time.timestamp_millis() >= cutoff);

            registry.locks.entry(user_id.to_owned()).or_default().clone()
        };

        AccountOperation {
            _guard: lock.lock_owned().await,
            operations: self.clone(),
            user_id: user_id.to_owned(),
        }
    }
}

/// Holds an account lock for the lifetime of a write operation.
#[must_use]
pub(crate) struct AccountOperation {
    _guard: OwnedMutexGuard<()>,
    operations: AccountOperations,
    user_id: String,
}

impl AccountOperation {
    /// Returns whether this sign-in is unexpired and began after the last deletion.
    ///
    /// Check this while holding the lock, after any network calls or lock waits.
    pub(crate) fn allows_login(&self, started_at: DateTime, expires_at: DateTime) -> bool {
        if expires_at <= DateTime::now() {
            return false;
        }

        let registry = self.operations.0.lock().unwrap_or_else(PoisonError::into_inner);

        registry.deleted_at.get(&self.user_id).is_none_or(|deleted_at| started_at > *deleted_at)
    }

    /// Rejects pending sign-ins that began before this completed deletion.
    pub(crate) fn mark_deleted(&self) {
        self.operations
            .0
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .deleted_at
            .insert(self.user_id.clone(), DateTime::now());
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn deletion_waits_for_an_admitted_writer_and_blocks_queued_old_logins() {
        let operations = AccountOperations::default();
        let writer = operations.lock("alice").await;
        let login_started = DateTime::now();
        let pending = operations.lock("alice");
        tokio::pin!(pending);

        assert!(
            tokio::time::timeout(std::time::Duration::from_millis(10), &mut pending).await.is_err()
        );

        drop(writer);
        let deletion = pending.await;
        deletion.mark_deleted();
        drop(deletion);

        let next = operations.lock("alice").await;

        assert!(!next.allows_login(login_started, DateTime::from_millis(i64::MAX)));
        assert!(
            next.allows_login(DateTime::from_millis(i64::MAX), DateTime::from_millis(i64::MAX))
        );
    }

    #[tokio::test]
    async fn account_locks_and_deletion_markers_do_not_affect_another_user() {
        let operations = AccountOperations::default();
        let deletion = operations.lock("alice").await;
        deletion.mark_deleted();
        let other = operations.lock("bob").await;

        assert!(other.allows_login(DateTime::from_millis(0), DateTime::from_millis(i64::MAX)));
    }
    #[tokio::test]
    async fn consumed_sign_ins_cannot_complete_after_their_state_expires() {
        let operations = AccountOperations::default();
        let operation = operations.lock("alice").await;

        assert!(!operation.allows_login(DateTime::from_millis(0), DateTime::from_millis(1)));
        assert!(operation.allows_login(DateTime::now(), DateTime::from_millis(i64::MAX)));
    }
}
