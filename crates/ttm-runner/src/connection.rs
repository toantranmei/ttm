//! The runner loop: heartbeat every 20 s, long-poll for jobs while there is a free slot, run jobs concurrently,
//! stop runs the server asks to stop (kill switch), pause while the agent's usage limit is reached, renew the
//! 12-hour runner token before it expires (or on a 401), and, with `auto_merge`, merge the head of each project's
//! merge queue.

use crate::SharedClient;
use crate::executor::{Executor, JobOutcome};
use crate::merge::{self, Merged};
use std::collections::{BTreeMap, HashMap};
use std::future::Future;
use std::path::PathBuf;
use std::pin::Pin;
use std::sync::Arc;
use std::time::Duration;
use time::OffsetDateTime;
use tokio::sync::watch;
use tokio::task::JoinSet;
use ttm_api::Client;
use ttm_api::factory::Heartbeat;

type PollFuture = Pin<Box<dyn Future<Output = ttm_api::Result<Option<ttm_api::factory::JobSpec>>> + Send>>;

pub const HEARTBEAT: Duration = Duration::from_secs(20);
/// Long-poll wait sent as `?wait=`: the server's maximum (it holds the request at most 25 s; the client timeout is 30 s).
pub const POLL_WAIT_SECS: u64 = 25;
/// Renew the runner token this long before it expires.
pub const RENEW_BEFORE: time::Duration = time::Duration::minutes(30);
/// Merge-queue pass every this many heartbeats (one minute).
const MERGE_EVERY: u32 = 3;

/// Registers again and returns the new runner client and token expiry.
pub type Renew = Arc<
    dyn Fn() -> Pin<Box<dyn Future<Output = anyhow::Result<(Client, Option<OffsetDateTime>)>> + Send>>
        + Send
        + Sync,
>;

/// What `auto_merge` needs: the user's client (reporting a merge needs `factory:write`, which the runner token lacks),
/// local clones by project key, and the worktree root.
#[derive(Clone)]
pub struct MergeSetup {
    pub user: Client,
    pub repos: BTreeMap<String, PathBuf>,
    pub worktree_root: PathBuf,
}

pub struct Runner {
    pub client: SharedClient,
    pub runner_id: String,
    pub capacity: usize,
    pub executor: Arc<Executor>,
    pub token_expires: Option<OffsetDateTime>,
    pub renew: Option<Renew>,
    pub merge: Option<MergeSetup>,
    /// `runner.toml`: when its modification time changes, the next heartbeat announces the local MCP servers again
    /// (names, allowed agents, `team_jobs`; never commands or environment).
    pub config_file: Option<PathBuf>,
}

/// Modification time of a file, if any.
pub fn mtime(path: &std::path::Path) -> Option<std::time::SystemTime> {
    std::fs::metadata(path).and_then(|m| m.modified()).ok()
}

/// What `runner.toml` announces to the server: local MCP servers and the projects with a local clone.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ConfigAnnounce {
    pub local_mcp: Vec<ttm_api::factory::LocalMcpAnnounce>,
    pub projects: Vec<String>,
}

/// `runner.toml` as seen when the loop starts: never, so the first heartbeat announces it.
pub const START_SEEN: Option<std::time::SystemTime> = None;

/// The announce to send now: `Some` when `runner.toml` changed since `seen` (updated in place).
pub fn config_changed(
    config_file: Option<&std::path::Path>,
    seen: &mut Option<std::time::SystemTime>,
) -> Option<ConfigAnnounce> {
    let file = config_file?;
    let now = mtime(file);
    if now == *seen {
        return None;
    }
    *seen = now;
    let dir = file.parent()?;
    let config = crate::RunnerConfig::load(dir).unwrap_or_default();
    Some(ConfigAnnounce {
        local_mcp: crate::local_mcp::announce(&config.mcp),
        projects: config.announced_projects(),
    })
}

/// Whether the token should be renewed now.
/// Whether the runner may take jobs given its `ttm-mcp` binary (the agents' `ttm` MCP server). Re-checked on every
/// turn of the loop (one `stat`): while it is missing the runner takes no job (each would be refused), and it takes
/// jobs again as soon as the binary is back.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct McpGate {
    pub ok: bool,
}

impl Default for McpGate {
    fn default() -> Self {
        Self { ok: true }
    }
}

impl McpGate {
    /// Re-checks `ttm_mcp`; returns what to tell the owner when it went missing or came back.
    pub fn update(&mut self, ttm_mcp: &std::path::Path) -> Option<String> {
        let now = crate::guard::check_ttm_mcp(ttm_mcp);
        let was = std::mem::replace(&mut self.ok, now.is_ok());
        match (was, now) {
            (true, Err(refused)) => Some(format!("{}; not taking jobs until it is back", refused.message)),
            (false, Ok(())) => Some(format!("ttm-mcp is back ({}); taking jobs again", ttm_mcp.display())),
            _ => None,
        }
    }
}

pub fn needs_renewal(expires: Option<OffsetDateTime>, now: OffsetDateTime) -> bool {
    expires.is_some_and(|at| at - now <= RENEW_BEFORE)
}

impl Runner {
    /// Runs until `shutdown` turns true or the server says to shut down.
    pub async fn run(mut self, mut shutdown: watch::Receiver<bool>) -> anyhow::Result<()> {
        let mut running: HashMap<String, watch::Sender<bool>> = HashMap::new();
        let mut tasks: JoinSet<JobOutcome> = JoinSet::new();
        let mut heartbeat = tokio::time::interval(HEARTBEAT);
        let mut beats: u32 = 0;
        let mut paused_until: Option<(String, tokio::time::Instant)> = None;
        let mut backoff = Duration::from_secs(1);
        let mut rejections = Rejections::default();
        let mut failure: Option<anyhow::Error> = None;
        // The long-poll in flight. It lives across loop turns: a heartbeat or a finished run must not drop a poll
        // the server may already have answered with a claimed job.
        let mut poll: Option<PollFuture> = None;
        // Nothing announced yet: the first heartbeat sends the current `runner.toml` and the features (a restart with
        // a still valid token does not register again, so changes made while it was down would be missed).
        let mut config_seen = START_SEEN;
        let mut mcp_gate = McpGate::default();
        tracing::info!(runner = %self.runner_id, capacity = self.capacity, "runner up");
        loop {
            if paused_until
                .as_ref()
                .is_some_and(|(_, at)| tokio::time::Instant::now() >= *at)
            {
                paused_until = None;
            }
            if let Some(change) = mcp_gate.update(&self.executor.ttm_mcp_exe) {
                tracing::warn!("{change}");
                self.executor.ui.eline(&change);
            }
            let can_take = running.len() < self.capacity && paused_until.is_none() && mcp_gate.ok;
            let client = self.client.get();
            if can_take && poll.is_none() {
                let (c, id) = (client.clone(), self.runner_id.clone());
                poll = Some(Box::pin(async move { c.poll_job(&id, POLL_WAIT_SECS).await }));
            }
            tokio::select! {
                _ = shutdown.changed() => {
                    if *shutdown.borrow() { break; }
                }
                _ = heartbeat.tick() => {
                    beats = beats.wrapping_add(1);
                    if needs_renewal(self.token_expires, OffsetDateTime::now_utc())
                        && let Err(err) = self.renew_token().await
                    {
                        tracing::warn!("could not renew the runner token: {err:#}");
                    }
                    let changed = config_changed(self.config_file.as_deref(), &mut config_seen);
                    let beat = Heartbeat {
                        running: running.keys().cloned().collect(),
                        capacity: self.capacity,
                        paused_until: paused_until.as_ref().map(|(s, _)| s.clone()),
                        local_mcp: changed.as_ref().map(|c| c.local_mcp.clone()),
                        features: changed.as_ref().map(|_| self.executor.features()),
                        projects: changed.map(|c| c.projects),
                    };
                    match self.client.get().heartbeat(&self.runner_id, &beat).await {
                        Ok(reply) => {
                            rejections.count = 0;
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
                        Err(err) if err.is_unauthorized() => {
                            if let Some(stop) = self.rejected(&mut rejections).await {
                                failure = Some(stop);
                                break;
                            }
                        }
                        Err(err) => tracing::warn!("heartbeat failed: {err}"),
                    }
                    if beats.is_multiple_of(MERGE_EVERY)
                        && let Some(setup) = self.merge.clone()
                    {
                        merge_pass(&setup).await;
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
                polled = async { poll.as_mut().expect("guarded by is_some").await }, if poll.is_some() => {
                    poll = None;
                    match polled {
                        Ok(Some(job)) => {
                            backoff = Duration::from_secs(1);
                            if let Err(err) = client.accept_job(&job.run_id).await {
                                tracing::warn!("could not accept run {}: {err}", job.run_id);
                                continue;
                            }
                            let (tx, rx) = watch::channel(false);
                            running.insert(job.run_id.clone(), tx);
                            let executor = self.executor.clone();
                            tasks.spawn(async move { executor.run(job, rx).await });
                        }
                        Ok(None) => backoff = Duration::from_secs(1),
                        Err(err) if err.is_unauthorized() => {
                            if let Some(stop) = self.rejected(&mut rejections).await {
                                failure = Some(stop);
                                break;
                            }
                        }
                        Err(err) => {
                            tracing::warn!("job poll failed: {err}; retrying in {backoff:?}");
                            tokio::time::sleep(backoff).await;
                            backoff = (backoff * 2).min(Duration::from_secs(60));
                        }
                    }
                }
                _ = tokio::time::sleep(Duration::from_secs(1)), if poll.is_none() => {}
            }
        }
        for tx in running.values() {
            let _ = tx.send(true);
        }
        while tasks.join_next().await.is_some() {}
        failure.map_or(Ok(()), Err)
    }

    /// The server rejected the runner token (expired, or the runner was turned off or removed on the web, which
    /// revokes it). Registers again at most once a minute; a turned-off runner then gets `shutdown` on the next
    /// heartbeat and stops. Returns an error to stop when renewal is refused or rejections keep coming.
    async fn rejected(&mut self, state: &mut Rejections) -> Option<anyhow::Error> {
        state.count += 1;
        if state.count > MAX_REJECTIONS {
            return Some(anyhow::anyhow!(
                "the server keeps rejecting this runner's token; it may have been removed on the web. Run `ttm runner up` again"
            ));
        }
        if state.last_renew.is_some_and(|at| at.elapsed() < RENEW_SPACING) {
            return None;
        }
        state.last_renew = Some(tokio::time::Instant::now());
        match self.renew_token().await {
            Ok(()) => None,
            Err(err) if is_auth_error(&err) => {
                Some(err.context("signing the runner in again was refused; run `ttm login`"))
            }
            Err(err) => {
                tracing::warn!("could not renew the runner token: {err:#}");
                None
            }
        }
    }

    /// Registers again with the person's sign-in (a runner token cannot register).
    async fn renew_token(&mut self) -> anyhow::Result<()> {
        let Some(renew) = self.renew.clone() else {
            anyhow::bail!("the runner token expired; restart `ttm runner up`");
        };
        let (client, expires) = renew().await?;
        self.client.set(client);
        self.token_expires = expires;
        tracing::info!("runner token renewed");
        Ok(())
    }
}

/// 401s in a row before the runner gives up.
pub const MAX_REJECTIONS: u32 = 3;
/// Least time between two re-registrations.
pub const RENEW_SPACING: Duration = Duration::from_secs(60);

#[derive(Debug, Default)]
struct Rejections {
    count: u32,
    last_renew: Option<tokio::time::Instant>,
}

/// 401/403 from the API anywhere in the error chain.
pub fn is_auth_error(err: &anyhow::Error) -> bool {
    err.chain()
        .filter_map(|e| e.downcast_ref::<ttm_api::ApiError>())
        .any(|e| matches!(e.status(), Some(401 | 403)))
}

/// One merge-queue pass over the configured projects: merge each head entry that may merge, report the result.
pub async fn merge_pass(setup: &MergeSetup) {
    for (key, repo) in &setup.repos {
        let project = match setup.user.find_project(key).await {
            Ok(Some(p)) => setup.user.project(&p.id).await.unwrap_or(p),
            Ok(None) => continue,
            Err(err) => {
                tracing::debug!("merge queue: cannot read project {key}: {err}");
                continue;
            }
        };
        let queue = match setup.user.merge_queue(&project.id).await {
            Ok(q) => q,
            Err(err) => {
                tracing::debug!("merge queue of {key}: {err}");
                continue;
            }
        };
        let Some(entry) = queue.into_iter().find(|e| e.can_merge && e.status == "ready") else {
            continue;
        };
        let base = project
            .default_branch
            .clone()
            .unwrap_or_else(|| ttm_core::worktree::default_branch(repo));
        let (repo, root, key2, entry2) = (
            repo.clone(),
            setup.worktree_root.clone(),
            key.clone(),
            entry.clone(),
        );
        let outcome =
            tokio::task::spawn_blocking(move || merge::merge_entry(&repo, &root, &key2, &base, &entry2))
                .await;
        let (merged, detail) = match outcome {
            Ok(Ok(Merged::Merged { commit })) => (true, Some(format!("merged as {commit}"))),
            Ok(Ok(Merged::Conflict { detail })) => (false, Some(detail)),
            Ok(Err(err)) => {
                // Push or fetch failed: not a conflict; try again next pass.
                tracing::warn!("merge of {} failed: {err:#}", entry.task_key);
                continue;
            }
            Err(err) => {
                tracing::warn!("merge task panicked: {err}");
                continue;
            }
        };
        match setup
            .user
            .merge_result(&entry.id, merged, detail.as_deref())
            .await
        {
            Ok(_) => tracing::info!(task = %entry.task_key, merged, "merge reported"),
            Err(err) => tracing::warn!("could not report the merge of {}: {err}", entry.task_key),
        }
    }
}

/// RFC 3339 → an instant (now when unparsable or past).
fn until_instant(rfc3339: &str) -> tokio::time::Instant {
    let now = tokio::time::Instant::now();
    OffsetDateTime::parse(rfc3339, &time::format_description::well_known::Rfc3339)
        .ok()
        .map(|at| at - OffsetDateTime::now_utc())
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
        let in_an_hour = (OffsetDateTime::now_utc() + time::Duration::hours(1))
            .format(&time::format_description::well_known::Rfc3339)
            .unwrap();
        assert!(until_instant(&in_an_hour) > now + Duration::from_secs(3500));
    }

    #[test]
    fn auth_errors_are_found_in_the_chain() {
        let api = ttm_api::ApiError::Problem(Box::new(ttm_api::Problem {
            status: 403,
            code: "forbidden".into(),
            title: "Forbidden".into(),
            detail: None,
            field: None,
            rule: None,
            params: None,
            required: None,
        }));
        let err = anyhow::Error::new(api).context("registering this runner");
        assert!(is_auth_error(&err));
        assert!(!is_auth_error(&anyhow::anyhow!("network down")));
    }

    #[cfg(unix)]
    #[test]
    fn no_jobs_while_ttm_mcp_is_missing() {
        use std::os::unix::fs::PermissionsExt;
        let dir = tempfile::tempdir().unwrap();
        let exe = dir.path().join("ttm-mcp");
        let mut gate = McpGate::default();
        let gone = gate.update(&exe).expect("reported once");
        assert!(gone.contains("not taking jobs"), "{gone}");
        assert!(!gate.ok);
        assert_eq!(gate.update(&exe), None, "said once, still paused");
        assert!(!gate.ok);
        std::fs::write(&exe, "").unwrap();
        std::fs::set_permissions(&exe, std::fs::Permissions::from_mode(0o755)).unwrap();
        assert!(gate.update(&exe).unwrap().contains("taking jobs again"));
        assert!(gate.ok);
        assert_eq!(gate.update(&exe), None);
    }

    #[test]
    fn renewal_window() {
        let now = OffsetDateTime::now_utc();
        assert!(!needs_renewal(None, now));
        assert!(!needs_renewal(Some(now + time::Duration::hours(2)), now));
        assert!(needs_renewal(Some(now + time::Duration::minutes(10)), now));
        assert!(needs_renewal(Some(now - time::Duration::minutes(1)), now));
    }

    #[test]
    fn local_mcp_and_projects_are_announced_again_when_the_file_changes() {
        let dir = tempfile::tempdir().unwrap();
        let file = dir.path().join("runner.toml");
        std::fs::write(
            &file,
            "[repos]\nAUR = \"/code/aurora\"\n\n[mcp.local.notes]\ncommand = \"notes\"\nallow = [\"dev\"]\n",
        )
        .unwrap();
        let mut seen = mtime(&file);
        assert!(config_changed(Some(&file), &mut seen).is_none(), "unchanged");
        let mut seen = None;
        let sent = config_changed(Some(&file), &mut seen).unwrap();
        assert_eq!(sent.local_mcp[0].name, "notes");
        assert_eq!(sent.projects, vec!["AUR".to_string()]);
        assert!(config_changed(None, &mut seen).is_none());
    }

    #[test]
    fn a_restarted_runner_announces_the_current_list_on_its_first_heartbeat() {
        let dir = tempfile::tempdir().unwrap();
        let file = dir.path().join("runner.toml");
        // changed while the runner was down (its token still valid: no registration on start)
        std::fs::write(&file, "[repos]\nAUR = \"/code/aurora\"\nNOTE = \"/code/notes\"\n").unwrap();
        let mut seen = START_SEEN;
        let sent = config_changed(Some(&file), &mut seen).expect("the first heartbeat announces");
        assert_eq!(sent.projects, vec!["AUR".to_string(), "NOTE".to_string()]);
        assert!(
            config_changed(Some(&file), &mut seen).is_none(),
            "then only on changes"
        );
    }
}
