//! Scheduler setup for background jobs.

use std::sync::Arc;

use rokbattles_api::db::ReportsStore;
use tokio::sync::Mutex;
use tokio_cron_scheduler::{Job, JobScheduler};
use tracing::{error, info, warn};

use crate::{error::JobsError, refresh_binds::refresh_claimed_governor_bindings};

/// Every 30 mins
pub const REFRESH_BINDS_CRON: &str = "0 0,30 * * * *";

/// Create the scheduler with the governor bind refresh job registered.
pub async fn build_scheduler(reports_store: ReportsStore) -> Result<JobScheduler, JobsError> {
    let scheduler = JobScheduler::new().await?;
    let reports_store = Arc::new(reports_store);
    let refresh_lock = Arc::new(Mutex::new(()));

    scheduler
        .add(Job::new_async(REFRESH_BINDS_CRON, move |_uuid, _lock| {
            let reports_store = Arc::clone(&reports_store);
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

    Ok(scheduler)
}

#[cfg(test)]
mod tests {
    use super::REFRESH_BINDS_CRON;

    #[test]
    fn refresh_binds_cron_runs_every_thirty_minutes_utc() {
        assert_eq!(REFRESH_BINDS_CRON, "0 0,30 * * * *");
    }
}
