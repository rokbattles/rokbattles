use std::sync::Arc;

use tokio::sync::Semaphore;

/// Admission is held by the blocking worker, even if its IPC caller goes away.
/// A stuck filesystem cannot create an unbounded queue of additional workers.
pub(crate) struct DirectoryOperations {
    workers: Arc<Semaphore>,
    discovery: Arc<Semaphore>,
}

impl Default for DirectoryOperations {
    fn default() -> Self {
        Self { workers: Arc::new(Semaphore::new(4)), discovery: Arc::new(Semaphore::new(1)) }
    }
}

impl DirectoryOperations {
    pub(crate) async fn run<T: Send + 'static>(
        &self,
        discover: bool,
        work: impl FnOnce() -> anyhow::Result<T> + Send + 'static,
    ) -> Result<T, String> {
        let discovery = if discover {
            Some(
                self.discovery
                    .clone()
                    .try_acquire_owned()
                    .map_err(|_busy| "Directory discovery is already running. Please wait.")?,
            )
        } else {
            None
        };
        let worker = self
            .workers
            .clone()
            .try_acquire_owned()
            .map_err(|_busy| "Directory operations are busy. Please wait and try again.")?;

        tauri::async_runtime::spawn_blocking(move || {
            let _worker = worker;
            let _discovery = discovery;

            work().map_err(|error| format!("{error:#}"))
        })
        .await
        .map_err(|error| error.to_string())?
    }
}

#[cfg(test)]
mod tests {
    use std::{sync::mpsc, time::Duration};

    use super::*;

    #[tokio::test]
    async fn blocked_discovery_keeps_executor_and_other_operations_responsive() {
        let operations = Arc::new(DirectoryOperations::default());
        let (started, ready) = tokio::sync::oneshot::channel();
        let (release, blocked) = mpsc::channel();
        let running = operations.clone();
        let task = tokio::spawn(async move {
            running
                .run(true, move || {
                    started.send(()).expect("notify test");
                    blocked.recv_timeout(Duration::from_secs(5))?;
                    Ok(())
                })
                .await
        });
        ready.await.expect("worker started");

        // A cancelled IPC future must not release admission for its still-running worker.
        task.abort();
        task.await.expect_err("caller was cancelled");
        assert!(operations.run(true, || Ok(())).await.is_err());

        let independent =
            tokio::time::timeout(Duration::from_secs(1), operations.run(false, || Ok(42)))
                .await
                .expect("independent command was responsive")
                .expect("command succeeded");
        assert_eq!(independent, 42);
        release.send(()).expect("release worker");
    }

    #[tokio::test]
    async fn full_worker_pool_rejects_work_without_starting_it() {
        let operations = DirectoryOperations::default();
        let _occupied = operations.workers.clone().acquire_many_owned(4).await.expect("permits");

        assert!(
            operations
                .run(false, || -> anyhow::Result<()> { panic!("must not run") })
                .await
                .is_err()
        );
        assert!(
            operations
                .run(true, || -> anyhow::Result<()> { panic!("must not run") })
                .await
                .is_err()
        );
        assert_eq!(operations.discovery.available_permits(), 1);
    }
}
