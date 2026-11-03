//! The runner loop: heartbeat every 20 s, long-poll for jobs while there is a free slot, run jobs concurrently,
//! stop runs the server asks to stop (kill switch), pause while the agent's usage limit is reached.

use crate::executor::{Executor, JobOutcome};
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::watch;
use tokio::task::JoinSet;
use ttm_api::Client;
use ttm_api::factory::Heartbeat;

pub const HEARTBEAT: Duration = Duration::from_secs(20);
pub const POLL_WAIT_SECS: u64 = 15;

pub struct Runner {
    pub client: Client,
    pub runner_id: String,
    pub capacity: usize,
    pub executor: Arc<Executor>,
}

impl Runner {
    /// Runs until `shutdown` turns true or the server says to shut down.
    pub async fn run(self, mut shutdown: watch::Receiver<bool>) -> anyhow::Result<()> {
        let mut running: HashMap<String, watch::Sender<bool>> = HashMap::new();
        let mut tasks: JoinSet<JobOutcome> = JoinSet::new();
        let mut heartbeat = tokio::time::interval(HEARTBEAT);
        let mut paused_until: Option<(String, tokio::time::Instant)> = None;
        let mut backoff = Duration::from_secs(1);
        tracing::info!(runner = %self.runner_id, capacity = self.capacity, "runner up");
        loop {
            if paused_until
                .as_ref()
                .is_some_and(|(_, at)| tokio::time::Instant::now() >= *at)
            {
                paused_until = None;
            }
            let can_take = running.len() < self.capacity && paused_until.is_none();
            tokio::select! {
                _ = shutdown.changed() => {
                    if *shutdown.borrow() { break; }
                }
                _ = heartbeat.tick() => {
                    let beat = Heartbeat {
                        running: running.keys().cloned().collect(),
                        capacity: self.capacity,
                        paused_until: paused_until.as_ref().map(|(s, _)| s.clone()),
                    };
                    match self.client.heartbeat(&self.runner_id, &beat).await {
                        Ok(reply) => {
                            for run in reply.stop_runs {
                                if let Some(tx) = running.get(&run) {
                                    tracing::info!(run = %run, "stop requested");
                                    let _ = tx.send(true);
                                }
                            }
                            if reply.shutdown {
                                tracing::info!("turned off from the web");
                                break;
                            }
                        }
                        Err(err) => tracing::warn!("heartbeat failed: {err}"),
                    }
                }
                Some(done) = tasks.join_next(), if !tasks.is_empty() => {
                    if let Ok(outcome) = done {
                        running.remove(&outcome.run_id);
                        tracing::info!(run = %outcome.run_id, status = %outcome.status, "run finished");
                        if let Some(until) = outcome.paused_until {
                            let wait = until_instant(&until);
                            tracing::warn!("agent usage limit reached; pausing until {until}");
                            paused_until = Some((until, wait));
                        }
                    }
                }
                polled = self.client.poll_job(&self.runner_id, POLL_WAIT_SECS), if can_take => {
                    match polled {
                        Ok(Some(job)) => {
                            backoff = Duration::from_secs(1);
                            if let Err(err) = self.client.accept_job(&job.run_id).await {
                                tracing::warn!("could not accept run {}: {err}", job.run_id);
                                continue;
                            }
                            let (tx, rx) = watch::channel(false);
                            running.insert(job.run_id.clone(), tx);
                            let executor = self.executor.clone();
                            tasks.spawn(async move { executor.run(job, rx).await });
                        }
                        Ok(None) => backoff = Duration::from_secs(1),
                        Err(err) => {
                            tracing::warn!("job poll failed: {err}; retrying in {backoff:?}");
                            tokio::time::sleep(backoff).await;
                            backoff = (backoff * 2).min(Duration::from_secs(60));
                        }
                    }
                }
                _ = tokio::time::sleep(Duration::from_secs(1)), if !can_take => {}
            }
        }
        for tx in running.values() {
            let _ = tx.send(true);
        }
        while tasks.join_next().await.is_some() {}
        Ok(())
    }
}

/// RFC 3339 → an instant (now when unparsable or past).
fn until_instant(rfc3339: &str) -> tokio::time::Instant {
    let now = tokio::time::Instant::now();
    time::OffsetDateTime::parse(rfc3339, &time::format_description::well_known::Rfc3339)
        .ok()
        .map(|at| at - time::OffsetDateTime::now_utc())
        .and_then(|d| u64::try_from(d.whole_seconds()).ok())
        .map(|secs| now + Duration::from_secs(secs))
        .unwrap_or(now)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pause_instant() {
        let now = tokio::time::Instant::now();
        assert!(until_instant("garbage") <= now + Duration::from_millis(5));
        let in_an_hour = (time::OffsetDateTime::now_utc() + time::Duration::hours(1))
            .format(&time::format_description::well_known::Rfc3339)
            .unwrap();
        assert!(until_instant(&in_an_hour) > now + Duration::from_secs(3500));
    }
}
