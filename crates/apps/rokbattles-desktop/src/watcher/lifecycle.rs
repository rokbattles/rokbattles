use std::future::Future;

use tokio::sync::watch;

/// Dropping the future cancels waits, not the watcher's state. The caller owns checkpointing and
/// restoring any in-flight item before it exits. A closed channel also means the owner is gone.
pub(super) async fn until_shutdown<T>(
    shutdown: &mut watch::Receiver<bool>,
    work: impl Future<Output = T>,
) -> Option<T> {
    if *shutdown.borrow() {
        return None;
    }
    tokio::select! {
        biased;
        _ = shutdown.changed() => None,
        result = work => Some(result),
    }
}

#[cfg(test)]
mod tests {
    use std::{cell::Cell, future::pending};

    use super::*;

    #[tokio::test]
    async fn completed_work_returns_its_result() {
        let (_tx, mut rx) = watch::channel(false);
        assert_eq!(until_shutdown(&mut rx, async { 42 }).await, Some(42));
    }

    #[tokio::test]
    async fn shutdown_wins_before_starting_ready_work() {
        let (tx, mut rx) = watch::channel(false);
        tx.send(true).expect("send shutdown");
        let started = Cell::new(false);
        assert!(until_shutdown(&mut rx, async { started.set(true) }).await.is_none());
        assert!(!started.get());
    }

    #[tokio::test]
    async fn shutdown_cancels_pending_work_and_drops_its_guard() {
        struct OnDrop<'a>(&'a Cell<bool>);
        impl Drop for OnDrop<'_> {
            fn drop(&mut self) {
                self.0.set(true);
            }
        }
        let dropped = Cell::new(false);
        let (tx, mut rx) = watch::channel(false);
        let (started_tx, started_rx) = tokio::sync::oneshot::channel();
        let work = async {
            let _guard = OnDrop(&dropped);
            started_tx.send(()).expect("signal work started");
            pending::<()>().await;
        };
        let stop = async {
            started_rx.await.expect("wait for work");
            tx.send(true).expect("send shutdown");
        };
        let (result, ()) = tokio::join!(until_shutdown(&mut rx, work), stop);
        assert!(result.is_none());
        assert!(dropped.get());
    }

    #[tokio::test]
    async fn dropping_owner_cancels_pending_work() {
        let (tx, mut rx) = watch::channel(false);
        drop(tx);
        assert!(until_shutdown(&mut rx, pending::<()>()).await.is_none());
    }
}
