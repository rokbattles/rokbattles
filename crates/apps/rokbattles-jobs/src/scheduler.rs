//! Scheduler setup for background jobs.

use std::{future::Future, sync::Arc};

use rokbattles_api::db::ReportsStore;
use tokio::sync::Mutex;
use tokio_cron_scheduler::{Job, JobScheduler};
use tracing::{error, info, warn};

use crate::{
    error::JobsError, precompute_barbarian::precompute_barbarian_data,
    precompute_barbarianfort::precompute_barbarian_fort_data,
    precompute_baulur::precompute_baulur_data,
    precompute_cmdr_pairings_v2::precompute_commander_pairings_v2_data,
    precompute_drastc::precompute_drastc_data,
    precompute_kahar_treasure::precompute_kahar_treasure_data,
    precompute_karuak_ceremony::precompute_karuak_ceremony_data,
    refresh_binds::refresh_claimed_governor_bindings,
};

/// Every 30 mins
pub const REFRESH_BINDS_CRON: &str = "0 */30 * * * *";
/// Every 8 hours
pub const PRECOMPUTE_BARBARIAN_CRON: &str = "0 0 */8 * * *";
/// Every 8 hours
pub const PRECOMPUTE_BARBARIAN_FORT_CRON: &str = "0 0 */8 * * *";
/// Every 8 hours
pub const PRECOMPUTE_BAULUR_CRON: &str = "0 0 */8 * * *";
/// Every 8 hours
pub const PRECOMPUTE_KAHAR_TREASURE_CRON: &str = "0 0 */8 * * *";
/// Every 8 hours
pub const PRECOMPUTE_KARUAK_CEREMONY_CRON: &str = "0 0 */8 * * *";
/// Every 8 hours
pub const PRECOMPUTE_DRASTC_CRON: &str = "0 0 */8 * * *";
/// Every 8 hours
pub const PRECOMPUTE_COMMANDER_PAIRINGS_V2_CRON: &str = "0 0 */8 * * *";

/// Create the scheduler with the governor bind refresh job registered.
pub async fn build_scheduler(reports_store: ReportsStore) -> Result<JobScheduler, JobsError> {
    let scheduler = JobScheduler::new().await?;
    let reports_store = Arc::new(reports_store);
    let refresh_lock = Arc::new(Mutex::new(()));
    let barbarian_lock = Arc::new(Mutex::new(()));
    let barbarian_fort_lock = Arc::new(Mutex::new(()));
    let baulur_lock = Arc::new(Mutex::new(()));
    let kahar_treasure_lock = Arc::new(Mutex::new(()));
    let karuak_ceremony_lock = Arc::new(Mutex::new(()));
    let drastc_lock = Arc::new(Mutex::new(()));
    let commander_pairings_v2_lock = Arc::new(Mutex::new(()));

    add_locked_job(
        &scheduler,
        REFRESH_BINDS_CRON,
        Arc::clone(&reports_store),
        refresh_lock,
        "governor bind refresh is already running; skipping this tick",
        |reports_store| async move {
            match refresh_claimed_governor_bindings(&reports_store).await {
                Ok(stats) => {
                    info!(
                        governors_seen = stats.governors_seen,
                        governors_refreshed = stats.governors_refreshed,
                        claims_matched = stats.claims_matched,
                        claims_updated = stats.claims_updated,
                        "refreshed claimed governor binds"
                    );
                }
                Err(error) => {
                    error!(%error, "failed to refresh claimed governor binds");
                }
            }
        },
    )
    .await?;

    add_locked_job(
        &scheduler,
        PRECOMPUTE_KARUAK_CEREMONY_CRON,
        Arc::clone(&reports_store),
        karuak_ceremony_lock,
        "Karuak Ceremony precompute is already running; skipping this tick",
        |reports_store| async move {
            match precompute_karuak_ceremony_data(&reports_store).await {
                Ok(stats) => info!(
                    documents_read = stats.documents_read,
                    results_counted = stats.results_counted,
                    documents_written = stats.documents_written,
                    "precomputed Karuak Ceremony data"
                ),
                Err(error) => error!(%error, "failed to precompute Karuak Ceremony data"),
            }
        },
    )
    .await?;

    add_locked_job(
        &scheduler,
        PRECOMPUTE_BARBARIAN_CRON,
        Arc::clone(&reports_store),
        barbarian_lock,
        "barbarian precompute is already running; skipping this tick",
        |reports_store| async move {
            match precompute_barbarian_data(&reports_store).await {
                Ok(stats) => {
                    info!(
                        documents_read = stats.documents_read,
                        reports_counted = stats.reports_counted,
                        documents_written = stats.documents_written,
                        "precomputed barbarian data"
                    );
                }
                Err(error) => {
                    error!(%error, "failed to precompute barbarian data");
                }
            }
        },
    )
    .await?;

    add_locked_job(
        &scheduler,
        PRECOMPUTE_BARBARIAN_FORT_CRON,
        Arc::clone(&reports_store),
        barbarian_fort_lock,
        "barbarian fort precompute is already running; skipping this tick",
        |reports_store| async move {
            match precompute_barbarian_fort_data(&reports_store).await {
                Ok(stats) => {
                    info!(
                        documents_read = stats.documents_read,
                        reports_counted = stats.reports_counted,
                        documents_written = stats.documents_written,
                        "precomputed barbarian fort data"
                    );
                }
                Err(error) => {
                    error!(%error, "failed to precompute barbarian fort data");
                }
            }
        },
    )
    .await?;

    add_locked_job(
        &scheduler,
        PRECOMPUTE_BAULUR_CRON,
        Arc::clone(&reports_store),
        baulur_lock,
        "Baulur precompute is already running; skipping this tick",
        |reports_store| async move {
            match precompute_baulur_data(&reports_store).await {
                Ok(stats) => {
                    info!(
                        documents_read = stats.documents_read,
                        results_counted = stats.results_counted,
                        documents_written = stats.documents_written,
                        "precomputed Baulur data"
                    );
                }
                Err(error) => {
                    error!(%error, "failed to precompute Baulur data");
                }
            }
        },
    )
    .await?;

    add_locked_job(
        &scheduler,
        PRECOMPUTE_KAHAR_TREASURE_CRON,
        Arc::clone(&reports_store),
        kahar_treasure_lock,
        "Kahar treasure precompute is already running; skipping this tick",
        |reports_store| async move {
            match precompute_kahar_treasure_data(&reports_store).await {
                Ok(stats) => {
                    info!(
                        documents_read = stats.documents_read,
                        mails_counted = stats.mails_counted,
                        documents_written = stats.documents_written,
                        "precomputed Kahar treasure data"
                    );
                }
                Err(error) => {
                    error!(%error, "failed to precompute Kahar treasure data");
                }
            }
        },
    )
    .await?;

    add_locked_job(
        &scheduler,
        PRECOMPUTE_DRASTC_CRON,
        Arc::clone(&reports_store),
        drastc_lock,
        "DRASTC precompute is already running; skipping this tick",
        |reports_store| async move {
            match precompute_drastc_data(&reports_store).await {
                Ok(stats) => {
                    info!(
                        legendary_commanders = stats.legendary_commanders,
                        observed_pairings = stats.observed_pairings,
                        supported_pairings = stats.supported_pairings,
                        scored_pairings = stats.scored_pairings,
                        confidence_scored_pairings = stats.confidence_scored_pairings,
                        documents_written = stats.documents_written,
                        documents_stored = stats.documents_stored,
                        "precomputed DRASTC data"
                    );
                }
                Err(error) => {
                    error!(%error, "failed to precompute DRASTC data");
                }
            }
        },
    )
    .await?;

    add_locked_job(
        &scheduler,
        PRECOMPUTE_COMMANDER_PAIRINGS_V2_CRON,
        reports_store,
        commander_pairings_v2_lock,
        "compact commander pairings precompute is already running; skipping this tick",
        |reports_store| async move {
            match precompute_commander_pairings_v2_data(&reports_store).await {
                Ok(stats) => info!(
                    legendary_commanders = stats.legendary_commanders,
                    pairings = stats.pairings,
                    performance_points = stats.performance_points,
                    loadout_snapshots = stats.loadout_snapshots,
                    documents_written = stats.documents_written,
                    max_document_bytes = stats.max_document_bytes,
                    performance_seconds = stats.performance_seconds,
                    loadout_seconds = stats.loadout_seconds,
                    total_seconds = stats.total_seconds,
                    "precomputed compact commander pairings data"
                ),
                Err(error) => {
                    error!(%error, "failed to precompute compact commander pairings data")
                }
            }
        },
    )
    .await?;

    Ok(scheduler)
}

async fn add_locked_job<F, Fut>(
    scheduler: &JobScheduler,
    cron: &str,
    reports_store: Arc<ReportsStore>,
    lock: Arc<Mutex<()>>,
    already_running_message: &'static str,
    task: F,
) -> Result<(), JobsError>
where
    F: Fn(Arc<ReportsStore>) -> Fut + Clone + Send + Sync + 'static,
    Fut: Future<Output = ()> + Send + 'static,
{
    scheduler
        .add(Job::new_async(cron, move |_uuid, _lock| {
            let reports_store = Arc::clone(&reports_store);
            let lock = Arc::clone(&lock);
            let task = task.clone();

            Box::pin(async move {
                let Ok(_guard) = lock.try_lock_owned() else {
                    warn!("{}", already_running_message);
                    return;
                };

                task(reports_store).await;
            })
        })?)
        .await?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::{
        PRECOMPUTE_BARBARIAN_CRON, PRECOMPUTE_BARBARIAN_FORT_CRON, PRECOMPUTE_BAULUR_CRON,
        PRECOMPUTE_COMMANDER_PAIRINGS_V2_CRON, PRECOMPUTE_DRASTC_CRON,
        PRECOMPUTE_KAHAR_TREASURE_CRON, PRECOMPUTE_KARUAK_CEREMONY_CRON, REFRESH_BINDS_CRON,
    };

    #[test]
    fn refresh_binds_cron_runs_every_thirty_minutes_utc() {
        assert_eq!(REFRESH_BINDS_CRON, "0 */30 * * * *");
    }

    #[test]
    fn precompute_barbarian_fort_cron_runs_every_eight_hours_utc() {
        assert_eq!(PRECOMPUTE_BARBARIAN_FORT_CRON, "0 0 */8 * * *");
    }

    #[test]
    fn precompute_barbarian_cron_runs_every_eight_hours_utc() {
        assert_eq!(PRECOMPUTE_BARBARIAN_CRON, "0 0 */8 * * *");
    }

    #[test]
    fn precompute_baulur_cron_runs_every_eight_hours_utc() {
        assert_eq!(PRECOMPUTE_BAULUR_CRON, "0 0 */8 * * *");
    }

    #[test]
    fn precompute_kahar_treasure_cron_runs_every_eight_hours_utc() {
        assert_eq!(PRECOMPUTE_KAHAR_TREASURE_CRON, "0 0 */8 * * *");
    }

    #[test]
    fn precompute_drastc_cron_runs_every_eight_hours_utc() {
        assert_eq!(PRECOMPUTE_DRASTC_CRON, "0 0 */8 * * *");
    }

    #[test]
    fn compact_commander_pairings_cron_is_synchronized_with_drastc() {
        assert_eq!(PRECOMPUTE_COMMANDER_PAIRINGS_V2_CRON, PRECOMPUTE_DRASTC_CRON);
    }

    #[test]
    fn precompute_karuak_ceremony_cron_runs_every_eight_hours_utc() {
        assert_eq!(PRECOMPUTE_KARUAK_CEREMONY_CRON, "0 0 */8 * * *");
    }
}
