//! Scheduler setup for background jobs.

use std::sync::Arc;

use rokbattles_api::db::ReportsStore;
use tokio::sync::Mutex;
use tokio_cron_scheduler::{Job, JobScheduler};
use tracing::{error, info, warn};

use crate::{
    error::JobsError, precompute_barbarianfort::precompute_barbarian_fort_data,
    refresh_binds::refresh_claimed_governor_bindings,
};

/// Every 30 mins
pub const REFRESH_BINDS_CRON: &str = "0 0,30 * * * *";
/// Every 8 hours
pub const PRECOMPUTE_BARBARIAN_FORT_CRON: &str = "0 0 */8 * * *";

/// Create the scheduler with the governor bind refresh job registered.
pub async fn build_scheduler(reports_store: ReportsStore) -> Result<JobScheduler, JobsError> {
    let scheduler = JobScheduler::new().await?;
    let reports_store = Arc::new(reports_store);
    let refresh_lock = Arc::new(Mutex::new(()));
    let barbarian_fort_lock = Arc::new(Mutex::new(()));
    let refresh_reports_store = Arc::clone(&reports_store);

    scheduler
        .add(Job::new_async(REFRESH_BINDS_CRON, move |_uuid, _lock| {
            let reports_store = Arc::clone(&refresh_reports_store);
            let refresh_lock = Arc::clone(&refresh_lock);

            Box::pin(async move {
                let Ok(_guard) = refresh_lock.try_lock_owned() else {
                    warn!("governor bind refresh is already running; skipping this tick");
                    return;
                };

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
            })
        })?)
        .await?;

    let barbarian_fort_reports_store = Arc::clone(&reports_store);
    scheduler
        .add(Job::new_async(PRECOMPUTE_BARBARIAN_FORT_CRON, move |_uuid, _lock| {
            let reports_store = Arc::clone(&barbarian_fort_reports_store);
            let barbarian_fort_lock = Arc::clone(&barbarian_fort_lock);

            Box::pin(async move {
                let Ok(_guard) = barbarian_fort_lock.try_lock_owned() else {
                    warn!("barbarian fort precompute is already running; skipping this tick");
                    return;
                };

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
            })
        })?)
        .await?;

    Ok(scheduler)
}

#[cfg(test)]
mod tests {
    use super::{PRECOMPUTE_BARBARIAN_FORT_CRON, REFRESH_BINDS_CRON};

    #[test]
    fn refresh_binds_cron_runs_every_thirty_minutes_utc() {
        assert_eq!(REFRESH_BINDS_CRON, "0 0,30 * * * *");
    }

    #[test]
    fn precompute_barbarian_fort_cron_runs_every_eight_hours_utc() {
        assert_eq!(PRECOMPUTE_BARBARIAN_FORT_CRON, "0 0 */8 * * *");
    }
}
