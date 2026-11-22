//! Scheduler setup for background jobs.

use std::{fmt::Debug, future::Future, sync::Arc, time::Instant};

use rokbattles_api::db::ReportsStore;
use tokio::sync::{Mutex, RwLock};
use tokio_cron_scheduler::{Job, JobScheduler};
use tracing::{Instrument, debug, error, info, info_span, warn};

use crate::{
    error::JobsError,
    precompute_barbarian::precompute_barbarian_data,
    precompute_barbarianfort::precompute_barbarian_fort_data,
    precompute_baulur::precompute_baulur_data,
    precompute_cmdr_pairings_v2::{
        precompute_commander_pairings_v2_data, precompute_commander_pairings_v2_presoc_data,
    },
    precompute_drastc::{precompute_drastc_data, precompute_drastc_presoc_data},
    precompute_kahar_treasure::precompute_kahar_treasure_data,
    precompute_karuak_ceremony::precompute_karuak_ceremony_data,
    precompute_lohar_trial::precompute_lohar_trial_data,
    precompute_strange_incidents::precompute_strange_incidents_data,
    refresh_binds::refresh_claimed_governor_bindings,
};

// Every eight hours UTC.
const PRECOMPUTE_CRON: &str = "0 0 */8 * * *";

/// Every 30 minutes UTC.
pub const REFRESH_BINDS_CRON: &str = "0 */30 * * * *";

/// Cron scheduler that drains active jobs before returning from shutdown.
pub struct JobsScheduler {
    cron: JobScheduler,
    accepting_jobs: Arc<RwLock<bool>>,
}

impl JobsScheduler {
    /// Start invoking jobs on their schedules.
    pub async fn start(&self) -> Result<(), JobsError> {
        self.cron.start().await?;
        Ok(())
    }

    /// Close admission and wait for active jobs, including dependent stages, to finish.
    pub async fn shutdown(&mut self) -> Result<(), JobsError> {
        // Each active job holds a reader. The queued writer prevents new readers
        // while cron independently stops dispatching ticks.
        let close_admission = async {
            *self.accepting_jobs.write().await = false;
        };
        let ((), result) = tokio::join!(biased; close_admission, self.cron.shutdown());
        result?;
        Ok(())
    }

    async fn add_job<F, Fut, Stats>(
        &self,
        reports_store: &Arc<ReportsStore>,
        cron: &str,
        name: &'static str,
        task: F,
    ) -> Result<(), JobsError>
    where
        F: Fn(Arc<ReportsStore>) -> Fut + Clone + Send + Sync + 'static,
        Fut: Future<Output = Result<Stats, JobsError>> + Send + 'static,
        Stats: Debug + Send,
    {
        let reports_store = Arc::clone(reports_store);
        let accepting_jobs = Arc::clone(&self.accepting_jobs);
        let lock = Arc::new(Mutex::new(()));

        self.cron
            .add(Job::new_async(cron, move |_id, _scheduler| {
                let reports_store = Arc::clone(&reports_store);
                let accepting_jobs = Arc::clone(&accepting_jobs);
                let lock = Arc::clone(&lock);
                let task = task.clone();

                Box::pin(async move {
                    run_job(&accepting_jobs, &lock, || task(reports_store))
                        .instrument(info_span!("scheduled_job", job = name))
                        .await
                })
            })?)
            .await?;
        debug!(job = name, cron, "registered scheduled job");
        Ok(())
    }
}

/// Create the scheduler with all recurring jobs registered.
pub async fn build_scheduler(reports_store: ReportsStore) -> Result<JobsScheduler, JobsError> {
    let scheduler = JobsScheduler {
        cron: JobScheduler::new().await?,
        accepting_jobs: Arc::new(RwLock::new(true)),
    };
    let store = Arc::new(reports_store);

    scheduler
        .add_job(&store, REFRESH_BINDS_CRON, "refresh_binds", |store| async move {
            refresh_claimed_governor_bindings(&store).await
        })
        .await?;
    scheduler
        .add_job(&store, PRECOMPUTE_CRON, "karuak_ceremony", |store| async move {
            precompute_karuak_ceremony_data(&store).await
        })
        .await?;
    scheduler
        .add_job(&store, PRECOMPUTE_CRON, "strange_incidents", |store| async move {
            precompute_strange_incidents_data(&store).await
        })
        .await?;
    scheduler
        .add_job(&store, PRECOMPUTE_CRON, "lohar_trial", |store| async move {
            precompute_lohar_trial_data(&store).await
        })
        .await?;
    scheduler
        .add_job(&store, PRECOMPUTE_CRON, "barbarian", |store| async move {
            precompute_barbarian_data(&store).await
        })
        .await?;
    scheduler
        .add_job(&store, PRECOMPUTE_CRON, "barbarian_fort", |store| async move {
            precompute_barbarian_fort_data(&store).await
        })
        .await?;
    scheduler
        .add_job(&store, PRECOMPUTE_CRON, "baulur", |store| async move {
            precompute_baulur_data(&store).await
        })
        .await?;
    scheduler
        .add_job(&store, PRECOMPUTE_CRON, "kahar_treasure", |store| async move {
            precompute_kahar_treasure_data(&store).await
        })
        .await?;

    // Keep each season's scores and dependent pairings under the same job lock.
    scheduler
        .add_job(&store, PRECOMPUTE_CRON, "combat_lab_soc", |store| async move {
            run_combat_lab_cycle(
                precompute_drastc_data(&store),
                precompute_commander_pairings_v2_data(&store),
            )
            .await
        })
        .await?;
    scheduler
        .add_job(&store, PRECOMPUTE_CRON, "combat_lab_presoc", |store| async move {
            run_combat_lab_cycle(
                precompute_drastc_presoc_data(&store),
                precompute_commander_pairings_v2_presoc_data(&store),
            )
            .await
        })
        .await?;

    Ok(scheduler)
}

async fn run_job<Stats: Debug, Fut: Future<Output = Result<Stats, JobsError>>>(
    accepting_jobs: &RwLock<bool>,
    lock: &Mutex<()>,
    task: impl FnOnce() -> Fut,
) {
    let _admission = match accepting_jobs.try_read() {
        Ok(guard) if *guard => guard,
        _ => {
            debug!("shutdown in progress, skipping tick");
            return;
        }
    };
    let Ok(_job_lock) = lock.try_lock() else {
        warn!("previous run still active, skipping tick");
        return;
    };

    let started = Instant::now();
    info!("job started");
    match task().await {
        Ok(stats) => info!(elapsed_ms = started.elapsed().as_millis(), ?stats, "job completed"),
        Err(error) => error!(elapsed_ms = started.elapsed().as_millis(), %error, "job failed"),
    }
}

async fn run_combat_lab_cycle(
    drastc: impl Future<Output = Result<impl Debug, JobsError>>,
    pairings: impl Future<Output = Result<impl Debug, JobsError>>,
) -> Result<(), JobsError> {
    run_stage("drastc", drastc).await?;
    run_stage("commander_pairings", pairings).await
}

async fn run_stage(
    stage: &'static str,
    task: impl Future<Output = Result<impl Debug, JobsError>>,
) -> Result<(), JobsError> {
    let started = Instant::now();
    match task.await {
        Ok(stats) => {
            info!(stage, elapsed_ms = started.elapsed().as_millis(), ?stats, "job stage completed");
            Ok(())
        }
        Err(error) => {
            error!(stage, elapsed_ms = started.elapsed().as_millis(), %error, "job stage failed");
            Err(error)
        }
    }
}

#[cfg(test)]
mod tests {
    use std::{
        future::{poll_fn, ready},
        task::{Context, Poll, Waker},
    };

    use tokio::sync::oneshot;

    use super::*;

    #[test]
    fn recurring_jobs_keep_their_utc_schedules() {
        assert_eq!(REFRESH_BINDS_CRON, "0 */30 * * * *");
        assert_eq!(PRECOMPUTE_CRON, "0 0 */8 * * *");
    }

    #[tokio::test]
    async fn shutdown_rejects_ticks_and_waits_for_all_active_jobs() {
        let accepting_jobs = Arc::new(RwLock::new(true));
        let mut scheduler = JobsScheduler {
            cron: JobScheduler::new().await.expect("cron scheduler"),
            accepting_jobs: Arc::clone(&accepting_jobs),
        };
        scheduler.start().await.expect("start scheduler");
        let lock = Mutex::new(());
        let other_job_lock = Mutex::new(());
        let idle_job_lock = Mutex::new(());
        let (finish_job, wait_for_finish) = oneshot::channel();
        let (finish_other_job, wait_for_other_finish) = oneshot::channel();
        let mut context = Context::from_waker(Waker::noop());

        let mut job = Box::pin(run_job(&accepting_jobs, &lock, || async {
            wait_for_finish.await.expect("finish signal");
            Ok(())
        }));
        assert!(matches!(job.as_mut().poll(&mut context), Poll::Pending));
        run_job(&accepting_jobs, &lock, || -> std::future::Ready<Result<(), JobsError>> {
            panic!("overlapping tick ran");
        })
        .await;

        let mut other_job = Box::pin(run_job(&accepting_jobs, &other_job_lock, || async {
            wait_for_other_finish.await.expect("finish signal");
            Ok(())
        }));
        assert!(matches!(other_job.as_mut().poll(&mut context), Poll::Pending));

        let mut shutdown = Box::pin(scheduler.shutdown());
        assert!(matches!(shutdown.as_mut().poll(&mut context), Poll::Pending));
        run_job(
            &accepting_jobs,
            &idle_job_lock,
            || -> std::future::Ready<Result<(), JobsError>> {
                panic!("tick ran during shutdown");
            },
        )
        .await;

        finish_job.send(()).expect("active job still waiting");
        job.await;
        assert!(matches!(shutdown.as_mut().poll(&mut context), Poll::Pending));
        finish_other_job.send(()).expect("other job still waiting");
        other_job.await;
        shutdown.await.expect("shutdown");
        run_job(
            &accepting_jobs,
            &idle_job_lock,
            || -> std::future::Ready<Result<(), JobsError>> {
                panic!("tick ran after shutdown");
            },
        )
        .await;
    }

    #[tokio::test]
    async fn combat_lab_skips_pairings_when_scoring_fails() {
        let pairings = poll_fn(|_| -> Poll<Result<(), JobsError>> {
            panic!("pairings ran without new scores");
        });
        let result =
            run_combat_lab_cycle(ready(Err::<(), _>(JobsError::MissingDatabase)), pairings).await;

        assert!(matches!(result, Err(JobsError::MissingDatabase)));
    }
}
