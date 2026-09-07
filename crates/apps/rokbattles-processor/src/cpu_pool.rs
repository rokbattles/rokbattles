//! Bounded CPU work on Tokio's blocking threads.

use std::{num::NonZeroUsize, sync::Arc};

use tokio::sync::Semaphore;

use crate::error::ProcessorError;

pub(crate) struct CpuPool {
    slots: Arc<Semaphore>,
}

impl CpuPool {
    pub(crate) fn new(concurrency: NonZeroUsize) -> Self {
        Self { slots: Arc::new(Semaphore::new(concurrency.get())) }
    }

    pub(crate) async fn run<T: Send + 'static>(
        &self,
        work: impl FnOnce() -> Result<T, ProcessorError> + Send + 'static,
    ) -> Result<T, ProcessorError> {
        let permit = Arc::clone(&self.slots).acquire_owned().await?;
        let span = tracing::Span::current();
        tokio::task::spawn_blocking(move || {
            // Running blocking work cannot be aborted. Keep its slot even if the
            // awaiting batch is dropped and a subsequent batch starts.
            let _permit = permit;
            span.in_scope(work)
        })
        .await?
    }
}

#[cfg(test)]
mod tests {
    use std::{
        sync::{
            atomic::{AtomicUsize, Ordering},
            mpsc,
        },
        time::Duration,
    };

    use futures::FutureExt;

    use super::*;

    #[tokio::test(flavor = "current_thread")]
    async fn runs_in_parallel_with_a_bound_and_leaves_runtime_responsive() {
        let pool = Arc::new(CpuPool::new(NonZeroUsize::new(2).unwrap()));
        let active = Arc::new(AtomicUsize::new(0));
        let peak = Arc::new(AtomicUsize::new(0));
        let (started_tx, mut started_rx) = tokio::sync::mpsc::unbounded_channel();
        let runtime_thread = std::thread::current().id();
        let mut releases = Vec::new();
        let mut tasks = Vec::new();
        for _ in 0..6 {
            let pool = Arc::clone(&pool);
            let active = Arc::clone(&active);
            let peak = Arc::clone(&peak);
            let started = started_tx.clone();
            let (release, wait) = mpsc::channel();
            releases.push(release);
            tasks.push(tokio::spawn(async move {
                pool.run(move || {
                    assert_ne!(std::thread::current().id(), runtime_thread);
                    let running = active.fetch_add(1, Ordering::SeqCst) + 1;
                    peak.fetch_max(running, Ordering::SeqCst);
                    started.send(()).unwrap();
                    wait.recv_timeout(Duration::from_secs(5)).unwrap();
                    active.fetch_sub(1, Ordering::SeqCst);
                    Ok(())
                })
                .await
            }));
        }
        for _ in 0..2 {
            tokio::time::timeout(Duration::from_secs(2), started_rx.recv()).await.unwrap().unwrap();
        }
        // This timer must run while both blocking workers are held at the gate.
        tokio::time::sleep(Duration::from_millis(20)).await;
        assert_eq!(active.load(Ordering::SeqCst), 2);
        assert!(started_rx.try_recv().is_err());
        for release in releases {
            release.send(()).unwrap();
        }
        for task in tasks {
            task.await.unwrap().unwrap();
        }
        assert_eq!(peak.load(Ordering::SeqCst), 2);
    }

    #[tokio::test(flavor = "current_thread")]
    async fn cancellation_holds_slot_until_blocking_work_finishes() {
        let pool = Arc::new(CpuPool::new(NonZeroUsize::new(1).unwrap()));
        let (started, running) = tokio::sync::oneshot::channel();
        let (release, wait) = mpsc::channel();
        let first_pool = Arc::clone(&pool);
        let first = tokio::spawn(async move {
            first_pool
                .run(move || {
                    started.send(()).unwrap();
                    wait.recv_timeout(Duration::from_secs(5)).unwrap();
                    Ok(())
                })
                .await
        });
        tokio::time::timeout(Duration::from_secs(2), running).await.unwrap().unwrap();
        first.abort();
        assert!(first.await.unwrap_err().is_cancelled());
        assert_eq!(pool.slots.available_permits(), 0);

        let second = pool.run(|| Ok(42));
        tokio::pin!(second);
        assert!(second.as_mut().now_or_never().is_none());
        release.send(()).unwrap();
        assert_eq!(
            tokio::time::timeout(Duration::from_secs(2), second).await.unwrap().unwrap(),
            42
        );
    }

    #[tokio::test]
    async fn cancelling_a_waiter_does_not_run_its_work_or_leak_capacity() {
        let pool = CpuPool::new(NonZeroUsize::new(1).unwrap());
        let permit = pool.slots.acquire().await.unwrap();
        {
            let waiting = pool.run::<()>(|| panic!("cancelled waiter must never be scheduled"));
            tokio::pin!(waiting);
            assert!(waiting.as_mut().now_or_never().is_none());
        }
        drop(permit);
        assert_eq!(pool.run(|| Ok(42)).await.unwrap(), 42);
    }

    #[tokio::test]
    async fn errors_and_panics_release_capacity() {
        let pool = CpuPool::new(NonZeroUsize::new(1).unwrap());
        let error = pool.run::<()>(|| Err(ProcessorError::MissingField("mail"))).await.unwrap_err();
        assert!(matches!(error, ProcessorError::MissingField("mail")));
        let error = pool.run::<()>(|| panic!("test worker panic")).await.unwrap_err();
        assert!(matches!(error, ProcessorError::PreparationTask(error) if error.is_panic()));
        assert_eq!(pool.run(|| Ok(42)).await.unwrap(), 42);
    }
}
