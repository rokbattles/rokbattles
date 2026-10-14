//! Serializes watcher lifecycle operations from frontend, tray, and updater requests.
//!
//! The manager owns at most one watcher. Once a command is queued, cancelling its
//! caller does not cancel the operation. Pausing, reprocessing, and installation
//! all wait for the current watcher to finish before changing ownership.
//!
//! A stop timeout leaves the worker in `Stopping` and fails pause, reprocess, or
//! installation requests. Exit continues waiting for that worker; a timeout never
//! permits a replacement watcher, upload-state reset, or installer to run alongside it.

use std::{
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
    time::Duration,
};

use tauri::AppHandle;
use tokio::sync::{mpsc, oneshot, watch};

use crate::watcher::{
    WatcherTask, emit_log, reset_upload_state, runtime::TaskEvent, spawn_watcher,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum WatcherStatus {
    Starting,
    Running,
    Paused,
    Stopping,
    Failed,
    Updating,
    Exiting,
}

enum Operation {
    Start,
    Pause,
    Toggle,
    Reprocess,
    Exit,
    InstallUpdate {
        install: Box<dyn FnOnce() -> anyhow::Result<()> + Send>,
        restart: Box<dyn FnOnce() + Send>,
    },
}

struct Request {
    operation: Operation,
    reply: oneshot::Sender<Result<(), String>>,
}

/// Sends lifecycle requests to the task that owns the watcher.
pub(crate) struct WatcherManager {
    commands: mpsc::Sender<Request>,
    status: watch::Receiver<WatcherStatus>,
    exiting: watch::Sender<bool>,
    exit_ready: Arc<AtomicBool>,
}

impl WatcherManager {
    pub(crate) fn new(app: &AppHandle) -> Self {
        let start_app = app.clone();
        let reset_app = app.clone();
        let tray_app = app.clone();
        let log_app = app.clone();

        Self::with_services(
            move || spawn_watcher(&start_app),
            move || reset_upload_state(&reset_app),
            move |status| {
                crate::tray::refresh_tray_menu(&tray_app, status);
                crate::shell::notify_capture_changed(&tray_app);
            },
            move |message| emit_log(&log_app, message),
        )
    }

    fn with_services(
        start: impl Fn() -> WatcherTask + Send + 'static,
        reset: impl Fn() -> anyhow::Result<()> + Send + Sync + 'static,
        changed: impl Fn(WatcherStatus) + Send + Sync + 'static,
        report: impl Fn(String) + Send + 'static,
    ) -> Self {
        let (commands, receiver) = mpsc::channel(32);
        let (status_sender, status) = watch::channel(WatcherStatus::Starting);
        let (exiting, _) = watch::channel(false);
        let exit_ready = Arc::new(AtomicBool::new(false));
        let actor_exit_ready = Arc::clone(&exit_ready);
        let actor_exiting = exiting.clone();

        // The actor owns transitions even if an invoking frontend command is cancelled.
        tauri::async_runtime::spawn(async move {
            run_manager(
                receiver,
                LifecycleSignals {
                    status: status_sender,
                    exiting: actor_exiting,
                    exit_ready: actor_exit_ready,
                },
                start,
                Arc::new(reset),
                changed,
                report,
            )
            .await;
        });

        Self { commands, status, exiting, exit_ready }
    }

    pub(crate) fn status(&self) -> WatcherStatus {
        *self.status.borrow()
    }

    pub(crate) fn exit_signal(&self) -> watch::Receiver<bool> {
        self.exiting.subscribe()
    }

    /// Marks exit as requested and returns whether this is the first request.
    pub(crate) fn mark_exit_requested(&self) -> bool {
        !self.exiting.send_replace(true)
    }

    pub(crate) fn exit_ready(&self) -> bool {
        self.exit_ready.load(Ordering::Acquire)
    }

    async fn request(&self, operation: Operation) -> Result<(), String> {
        let (reply, receiver) = oneshot::channel();
        self.commands
            .send(Request { operation, reply })
            .await
            .map_err(|error| format!("Watcher manager is unavailable: {error}"))?;

        receiver
            .await
            .map_err(|error| format!("Watcher manager stopped during the operation: {error}"))?
    }

    /// Requests a watcher if none exists, without waiting for its initial scan.
    pub(crate) async fn start(&self) -> Result<(), String> {
        self.request(Operation::Start).await
    }

    /// Requests a stop and waits for the worker within the stop budget.
    ///
    /// A timeout returns an error while the manager continues to own the worker.
    /// Its status remains [`WatcherStatus::Stopping`] until it finishes.
    pub(crate) async fn pause(&self) -> Result<(), String> {
        self.request(Operation::Pause).await
    }

    pub(crate) async fn toggle(&self) -> Result<(), String> {
        self.request(Operation::Toggle).await
    }

    /// Stops the watcher, resets upload history, and starts scanning again.
    pub(crate) async fn reprocess(&self) -> Result<(), String> {
        self.request(Operation::Reprocess).await
    }

    /// Waits for the watcher to finish before allowing the application to exit.
    pub(crate) async fn exit(&self) -> Result<(), String> {
        self.mark_exit_requested();
        self.request(Operation::Exit).await
    }

    /// Stops the watcher before installation and requests a restart on success.
    ///
    /// If installation fails, scanning resumes only if the watcher was active
    /// before the request and application exit has not been requested.
    pub(crate) async fn install_update(
        &self,
        install: impl FnOnce() -> anyhow::Result<()> + Send + 'static,
        restart: impl FnOnce() + Send + 'static,
    ) -> Result<(), String> {
        self.request(Operation::InstallUpdate {
            install: Box::new(install),
            restart: Box::new(restart),
        })
        .await
    }
}

async fn stop_task(
    task: &mut Option<WatcherTask>,
    publish: &impl Fn(WatcherStatus),
) -> Result<(), String> {
    let Some(owner) = task.as_mut() else {
        publish(WatcherStatus::Paused);
        return Ok(());
    };

    publish(WatcherStatus::Stopping);
    owner.request_stop();

    let result = match tokio::time::timeout(Duration::from_secs(3), owner.join()).await {
        Ok(result) => result,
        Err(_) => {
            // Blocking filesystem I/O cannot safely be aborted. Keep ownership until joined.
            return Err(
                "Watcher is still stopping; waiting for filesystem work to finish".to_string()
            );
        }
    };
    *task = None;

    publish(if result.is_ok() { WatcherStatus::Paused } else { WatcherStatus::Failed });

    result.map_err(|error| format!("Watcher stopped with an error: {error:#}"))
}

struct LifecycleSignals {
    status: watch::Sender<WatcherStatus>,
    exiting: watch::Sender<bool>,
    exit_ready: Arc<AtomicBool>,
}

async fn run_manager(
    mut commands: mpsc::Receiver<Request>,
    signals: LifecycleSignals,
    start: impl Fn() -> WatcherTask,
    reset: Arc<impl Fn() -> anyhow::Result<()> + Send + Sync + 'static>,
    changed: impl Fn(WatcherStatus),
    report: impl Fn(String),
) {
    let LifecycleSignals { status, exiting, exit_ready } = signals;
    let publish = |next| {
        status.send_replace(next);
        changed(next);
    };
    let mut task = if *exiting.borrow() { None } else { Some(start()) };

    loop {
        let request = if let Some(owner) = task.as_mut() {
            tokio::select! {
                biased;

                event = owner.next_event() => {
                    match event {
                        TaskEvent::Ready => {
                            if *status.borrow() == WatcherStatus::Starting {
                                publish(WatcherStatus::Running);
                            }
                        }
                        TaskEvent::Finished(result) => {
                            task = None;
                            let stopping = *status.borrow() == WatcherStatus::Stopping;

                            match result {
                                Ok(()) if stopping => publish(WatcherStatus::Paused),
                                Ok(()) => {
                                    report("Watcher stopped unexpectedly".to_string());
                                    publish(WatcherStatus::Failed);
                                }
                                Err(error) => {
                                    report(format!("Watcher failed: {error:#}"));
                                    publish(WatcherStatus::Failed);
                                }
                            }
                        }
                    }
                    continue;
                }
                request = commands.recv() => request,
            }
        } else {
            commands.recv().await
        };

        let Some(Request { operation, reply }) = request else {
            if let Some(mut owner) = task.take() {
                owner.request_stop();
                if let Err(error) = owner.join().await {
                    report(format!("Watcher failed during manager shutdown: {error:#}"));
                }
            }
            return;
        };

        if *exiting.borrow() && !matches!(&operation, Operation::Exit) {
            let _reply_result = reply.send(Err("Application is exiting".to_string()));
            continue;
        }

        let result = match operation {
            Operation::Toggle
                if matches!(*status.borrow(), WatcherStatus::Starting | WatcherStatus::Running) =>
            {
                stop_task(&mut task, &publish).await
            }
            Operation::Start | Operation::Toggle => {
                if *status.borrow() == WatcherStatus::Stopping {
                    Err("Watcher is still stopping".to_string())
                } else {
                    if task.is_none() {
                        publish(WatcherStatus::Starting);
                        task = Some(start());
                    }
                    Ok(())
                }
            }
            Operation::Pause => stop_task(&mut task, &publish).await,
            Operation::Reprocess => {
                if *exiting.borrow() {
                    Err("Application is exiting".to_string())
                } else if let Err(error) = stop_task(&mut task, &publish).await {
                    Err(error)
                } else {
                    let reset = Arc::clone(&reset);
                    let result = tauri::async_runtime::spawn_blocking(move || reset())
                        .await
                        .map_err(anyhow::Error::from)
                        .and_then(|result| result);

                    match result {
                        Ok(()) if !*exiting.borrow() => {
                            publish(WatcherStatus::Starting);
                            task = Some(start());
                            Ok(())
                        }
                        Ok(()) => Err("Application is exiting".to_string()),
                        Err(error) => {
                            publish(WatcherStatus::Failed);
                            Err(format!("Failed to reset upload state: {error:#}"))
                        }
                    }
                }
            }
            Operation::InstallUpdate { install, restart } => {
                let resume_on_failure =
                    matches!(*status.borrow(), WatcherStatus::Starting | WatcherStatus::Running);
                if *exiting.borrow() {
                    Err("Application is exiting".to_string())
                } else if let Err(error) = stop_task(&mut task, &publish).await {
                    Err(error)
                } else if *exiting.borrow() {
                    Err("Application is exiting".to_string())
                } else {
                    publish(WatcherStatus::Updating);
                    let result = tauri::async_runtime::spawn_blocking(install)
                        .await
                        .map_err(anyhow::Error::from)
                        .and_then(|result| result);

                    match result {
                        Ok(()) => {
                            exiting.send_replace(true);
                            exit_ready.store(true, Ordering::Release);
                            publish(WatcherStatus::Exiting);
                            restart();
                            Ok(())
                        }
                        Err(error) => {
                            if resume_on_failure && !*exiting.borrow() {
                                publish(WatcherStatus::Starting);
                                task = Some(start());
                            } else {
                                publish(WatcherStatus::Paused);
                            }

                            Err(format!("Update installation failed: {error:#}"))
                        }
                    }
                }
            }
            Operation::Exit => {
                if let Err(error) = stop_task(&mut task, &publish).await {
                    report(error);
                }

                // A slow disk may outlast the budget. Keep denying exit/restart and new owners
                // until it finishes; repeated ExitRequested events must not bypass this join.
                if let Some(mut owner) = task.take()
                    && let Err(error) = owner.join().await
                {
                    report(format!("Watcher failed during exit: {error:#}"));
                }

                exit_ready.store(true, Ordering::Release);
                publish(WatcherStatus::Exiting);
                Ok(())
            }
        };

        if let Err(error) = &result {
            report(error.clone());
        }

        let _reply_result = reply.send(result);
    }
}

#[cfg(test)]
mod tests {
    use std::sync::{Mutex, atomic::AtomicUsize};

    use super::*;

    #[derive(Default)]
    struct Owners {
        active: AtomicUsize,
        maximum: AtomicUsize,
        starts: AtomicUsize,
        resets: AtomicUsize,
    }

    struct ActiveOwner(Arc<Owners>);

    impl Drop for ActiveOwner {
        fn drop(&mut self) {
            self.0.active.fetch_sub(1, Ordering::SeqCst);
        }
    }

    fn owner(owners: Arc<Owners>) -> WatcherTask {
        WatcherTask::spawn(move |mut shutdown, ready| async move {
            let active = owners.active.fetch_add(1, Ordering::SeqCst) + 1;
            owners.maximum.fetch_max(active, Ordering::SeqCst);
            owners.starts.fetch_add(1, Ordering::SeqCst);
            let _owner = ActiveOwner(owners);
            let _ready_result = ready.send(());

            let _shutdown_result = shutdown.wait_for(|stopping| *stopping).await;

            Ok(())
        })
    }

    fn manager(owners: &Arc<Owners>) -> Arc<WatcherManager> {
        let start_owners = Arc::clone(owners);
        let reset_owners = Arc::clone(owners);

        Arc::new(WatcherManager::with_services(
            move || owner(Arc::clone(&start_owners)),
            move || {
                assert_eq!(
                    reset_owners.active.load(Ordering::SeqCst),
                    0,
                    "reset requires joined ownership"
                );
                reset_owners.resets.fetch_add(1, Ordering::SeqCst);
                Ok(())
            },
            |_| {},
            |_| {},
        ))
    }

    async fn wait_status(manager: &WatcherManager, expected: WatcherStatus) {
        let mut status = manager.status.clone();
        tokio::time::timeout(Duration::from_secs(5), status.wait_for(|status| *status == expected))
            .await
            .expect("lifecycle status arrives")
            .expect("manager remains available");
    }

    #[tokio::test]
    async fn startup_pause_is_not_overridden_and_concurrent_controls_have_one_owner() {
        let owners = Arc::new(Owners::default());
        let manager = manager(&owners);
        manager.pause().await.expect("pause during startup");

        assert_eq!(manager.status(), WatcherStatus::Paused);
        assert_eq!(owners.active.load(Ordering::SeqCst), 0);

        let mut controls = tokio::task::JoinSet::new();

        for index in 0..24 {
            let manager = Arc::clone(&manager);
            controls.spawn(async move {
                match index % 3 {
                    0 => manager.start().await,
                    1 => manager.pause().await,
                    _ => manager.reprocess().await,
                }
            });
        }

        while let Some(result) = controls.join_next().await {
            result.expect("control task").expect("serialized control succeeds");
        }

        manager.start().await.expect("final resume");
        wait_status(&manager, WatcherStatus::Running).await;
        manager.pause().await.expect("final pause");

        assert_eq!(owners.maximum.load(Ordering::SeqCst), 1);
        assert_eq!(owners.active.load(Ordering::SeqCst), 0);
        assert_eq!(owners.resets.load(Ordering::SeqCst), 8);
    }

    #[tokio::test]
    async fn task_failure_updates_status_and_does_not_block_a_new_owner() {
        let owners = Arc::new(Owners::default());
        let starts = Arc::new(AtomicUsize::new(0));
        let start_count = Arc::clone(&starts);
        let next_owners = Arc::clone(&owners);
        let messages = Arc::new(Mutex::new(Vec::new()));
        let reports = Arc::clone(&messages);
        let manager = WatcherManager::with_services(
            move || {
                if start_count.fetch_add(1, Ordering::SeqCst) == 0 {
                    WatcherTask::spawn(|_, _| async { anyhow::bail!("fixture storage failure") })
                } else {
                    owner(Arc::clone(&next_owners))
                }
            },
            || Ok(()),
            |_| {},
            move |message| reports.lock().expect("diagnostics").push(message),
        );
        wait_status(&manager, WatcherStatus::Failed).await;

        assert!(
            messages
                .lock()
                .expect("diagnostics")
                .iter()
                .any(|message| message.contains("fixture storage failure"))
        );

        manager.start().await.expect("retry failed watcher");
        wait_status(&manager, WatcherStatus::Running).await;
        manager.pause().await.expect("stop replacement");

        assert_eq!(starts.load(Ordering::SeqCst), 2);
    }

    #[tokio::test]
    async fn timed_out_stop_retains_ownership_until_join_before_reset_or_replacement() {
        let owners = Arc::new(Owners::default());
        let start_owners = Arc::clone(&owners);
        let reset_owners = Arc::clone(&owners);
        let (release, blocked) = std::sync::mpsc::channel();
        let gate = Arc::new(Mutex::new(Some(blocked)));
        let manager = Arc::new(WatcherManager::with_services(
            move || {
                let gate = gate.lock().expect("storage gate").take();
                let owners = Arc::clone(&start_owners);
                WatcherTask::spawn(move |mut shutdown, ready| async move {
                    let active = owners.active.fetch_add(1, Ordering::SeqCst) + 1;
                    owners.maximum.fetch_max(active, Ordering::SeqCst);
                    let _owner = ActiveOwner(owners);
                    let _ready_result = ready.send(());
                    let _shutdown_result = shutdown.wait_for(|stopping| *stopping).await;

                    if let Some(gate) = gate {
                        gate.recv().expect("finish blocked storage write");
                    }
                    Ok(())
                })
            },
            move || {
                assert_eq!(reset_owners.active.load(Ordering::SeqCst), 0);
                reset_owners.resets.fetch_add(1, Ordering::SeqCst);
                Ok(())
            },
            |_| {},
            |_| {},
        ));
        wait_status(&manager, WatcherStatus::Running).await;
        manager.pause().await.expect_err("slow storage must retain the stopping owner");
        manager.start().await.expect_err("resume must not create an overlapping owner");

        assert_eq!(manager.status(), WatcherStatus::Stopping);
        assert_eq!(owners.active.load(Ordering::SeqCst), 1);

        let reset_manager = Arc::clone(&manager);
        let mut reset = tokio::spawn(async move { reset_manager.reprocess().await });
        tokio::time::timeout(Duration::from_millis(30), &mut reset)
            .await
            .expect_err("reset waits for the old writer");

        assert_eq!(owners.resets.load(Ordering::SeqCst), 0);

        release.send(()).expect("storage completes");
        reset.await.expect("reset caller").expect("reset after joined writer");
        wait_status(&manager, WatcherStatus::Running).await;
        manager.pause().await.expect("stop replacement");

        assert_eq!(owners.resets.load(Ordering::SeqCst), 1);
        assert_eq!(owners.maximum.load(Ordering::SeqCst), 1);
    }

    #[tokio::test]
    async fn exit_during_cancelled_reprocess_waits_for_reset_and_prevents_restart() {
        let owners = Arc::new(Owners::default());
        let start_owners = Arc::clone(&owners);
        let (entered, reset_entered) = oneshot::channel();
        let entered = Mutex::new(Some(entered));
        let (release, blocked) = std::sync::mpsc::channel();
        let blocked = Mutex::new(blocked);
        let manager = Arc::new(WatcherManager::with_services(
            move || owner(Arc::clone(&start_owners)),
            move || {
                entered
                    .lock()
                    .expect("reset signal")
                    .take()
                    .expect("one reset")
                    .send(())
                    .expect("signal reset");
                blocked.lock().expect("reset gate").recv().expect("finish reset");
                Ok(())
            },
            |_| {},
            |_| {},
        ));
        wait_status(&manager, WatcherStatus::Running).await;
        let reset_manager = Arc::clone(&manager);
        let reset = tokio::spawn(async move { reset_manager.reprocess().await });
        reset_entered.await.expect("reset accepted");
        reset.abort();
        reset.await.expect_err("frontend stopped waiting");

        assert!(manager.mark_exit_requested());
        assert!(!manager.mark_exit_requested(), "repeated exit remains pending");
        assert!(!manager.exit_ready(), "exit cannot bypass a reset still writing");

        let exit_manager = Arc::clone(&manager);
        let exit = tokio::spawn(async move { exit_manager.exit().await });
        release.send(()).expect("finish reset");
        exit.await.expect("exit caller").expect("joined exit");
        manager.start().await.expect_err("exit permanently blocks new owners");

        assert!(manager.exit_ready());
        assert_eq!(owners.starts.load(Ordering::SeqCst), 1);
        assert_eq!(owners.active.load(Ordering::SeqCst), 0);
    }

    #[tokio::test]
    async fn accepted_update_still_restarts_if_the_caller_is_cancelled() {
        let owners = Arc::new(Owners::default());
        let manager = manager(&owners);
        wait_status(&manager, WatcherStatus::Running).await;
        let install_owners = Arc::clone(&owners);
        let (entered, installing) = oneshot::channel();
        let (release, blocked) = std::sync::mpsc::channel();
        let (restart, restarted) = oneshot::channel();
        let update_manager = Arc::clone(&manager);
        let update = tokio::spawn(async move {
            update_manager
                .install_update(
                    move || {
                        assert_eq!(
                            install_owners.active.load(Ordering::SeqCst),
                            0,
                            "join before installer can exit the process"
                        );
                        entered.send(()).expect("installation accepted");
                        blocked.recv().expect("installation completes");
                        Ok(())
                    },
                    move || {
                        restart.send(()).expect("restart owned by actor");
                    },
                )
                .await
        });
        installing.await.expect("installer started");
        update.abort();
        update.await.expect_err("updater caller cancelled");
        release.send(()).expect("complete installation");
        tokio::time::timeout(Duration::from_secs(5), restarted)
            .await
            .expect("restart independent of cancelled caller")
            .expect("restart callback");

        assert!(manager.exit_ready());
        assert_eq!(manager.status(), WatcherStatus::Exiting);
        manager.start().await.expect_err("cannot start after successful installation");
    }

    #[tokio::test]
    async fn failed_install_restores_a_running_watcher_but_preserves_user_pause() {
        for paused in [false, true] {
            let owners = Arc::new(Owners::default());
            let manager = manager(&owners);
            wait_status(&manager, WatcherStatus::Running).await;

            if paused {
                manager.pause().await.expect("user pause");
            }

            let error = manager
                .install_update(
                    || anyhow::bail!("fixture installer failure"),
                    || panic!("failed installation must not restart the application"),
                )
                .await
                .expect_err("installation failure is reported");

            assert!(error.contains("fixture installer failure"));

            if paused {
                assert_eq!(manager.status(), WatcherStatus::Paused);
                assert_eq!(owners.starts.load(Ordering::SeqCst), 1);
            } else {
                wait_status(&manager, WatcherStatus::Running).await;
                assert_eq!(owners.starts.load(Ordering::SeqCst), 2);
                manager.pause().await.expect("stop recovered watcher");
            }
        }
    }
}
