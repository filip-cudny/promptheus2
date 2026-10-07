mod detect;
mod git;
mod reload;
#[cfg(test)]
mod tests;

use std::collections::HashMap;
use std::ffi::OsString;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use async_trait::async_trait;
use serde::Serialize;

pub use detect::{detect, Detection, SyncRepo};
pub use git::{GitError, GitRunner};
pub use reload::{reload_changed, Reloaded};

const DEFAULT_TIMEOUT: Duration = Duration::from_secs(60);
const UNKNOWN_HOST: &str = "unknown";

pub type EnvProvider = Arc<dyn Fn() -> HashMap<String, String> + Send + Sync>;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Trigger {
    Startup,
    Periodic,
    Manual,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum SyncState {
    Off,
    Synced,
    Syncing,
    Conflict,
    Error,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct SyncStatus {
    pub state: SyncState,
    pub message: Option<String>,
    pub repo_path: Option<String>,
    pub branch: Option<String>,
    pub last_sync: Option<String>,
}

impl SyncStatus {
    fn initial() -> Self {
        Self {
            state: SyncState::Off,
            message: None,
            repo_path: None,
            branch: None,
            last_sync: None,
        }
    }
}

#[async_trait]
pub trait SyncHooks: Send + Sync {
    type Guard: Send;

    async fn lock_local(&self) -> Self::Guard;
    async fn reload(&self, guard: &mut Self::Guard, changed: &[PathBuf]);
    fn status_changed(&self, status: &SyncStatus);
}

#[derive(Debug, Clone, PartialEq)]
pub struct CycleReport {
    pub status: SyncStatus,
    pub changed: Vec<PathBuf>,
    pub entered_new_state: bool,
}

#[derive(Debug, Clone, PartialEq)]
pub enum SyncResult {
    AlreadySyncing,
    Paused,
    Completed(CycleReport),
}

struct Inner {
    status: SyncStatus,
    running: bool,
    paused: bool,
}

pub struct SettingsSync {
    config_dir: PathBuf,
    env_provider: EnvProvider,
    host: tokio::sync::OnceCell<String>,
    configured_host: Option<String>,
    timeout: Duration,
    inner: Arc<Mutex<Inner>>,
}

struct RunningGuard(Arc<Mutex<Inner>>);

impl Drop for RunningGuard {
    fn drop(&mut self) {
        if let Ok(mut inner) = self.0.lock() {
            inner.running = false;
        }
    }
}

enum Stop {
    Conflict(String),
    Error(String),
}

impl From<GitError> for Stop {
    fn from(e: GitError) -> Self {
        Stop::Error(e.to_string())
    }
}

impl SettingsSync {
    pub fn new(
        config_dir: PathBuf,
        env_provider: EnvProvider,
        host: Option<String>,
        timeout: Option<Duration>,
    ) -> Self {
        Self {
            config_dir,
            env_provider,
            host: tokio::sync::OnceCell::new(),
            configured_host: host,
            timeout: timeout.unwrap_or(DEFAULT_TIMEOUT),
            inner: Arc::new(Mutex::new(Inner {
                status: SyncStatus::initial(),
                running: false,
                paused: false,
            })),
        }
    }

    pub fn status(&self) -> SyncStatus {
        self.inner.lock().unwrap().status.clone()
    }

    pub async fn sync<H: SyncHooks>(&self, trigger: Trigger, hooks: &H) -> SyncResult {
        let previous = {
            let mut inner = self.inner.lock().unwrap();
            if inner.running {
                return SyncResult::AlreadySyncing;
            }
            if trigger == Trigger::Periodic && inner.paused {
                return SyncResult::Paused;
            }
            inner.running = true;
            inner.status.clone()
        };
        let _running = RunningGuard(self.inner.clone());

        self.publish(
            hooks,
            SyncStatus {
                state: SyncState::Syncing,
                message: None,
                ..previous.clone()
            },
        );

        let (status, changed) = self.run_cycle(hooks, &previous).await;
        let entered_new_state = status.state != previous.state;
        self.publish(hooks, status.clone());
        SyncResult::Completed(CycleReport {
            status,
            changed,
            entered_new_state,
        })
    }

    fn publish<H: SyncHooks>(&self, hooks: &H, status: SyncStatus) {
        self.inner.lock().unwrap().status = status.clone();
        hooks.status_changed(&status);
    }

    fn set_paused(&self, paused: bool) {
        self.inner.lock().unwrap().paused = paused;
    }

    async fn run_cycle<H: SyncHooks>(
        &self,
        hooks: &H,
        previous: &SyncStatus,
    ) -> (SyncStatus, Vec<PathBuf>) {
        let provider = self.env_provider.clone();
        let env = tokio::task::spawn_blocking(move || provider())
            .await
            .unwrap_or_default();
        let runner = GitRunner::new(env, self.timeout);

        let repo = match detect(&runner, &self.config_dir).await {
            Detection::Off => {
                log::info!("settings sync: no symlinked settings repo, sync is off");
                return (
                    SyncStatus {
                        state: SyncState::Off,
                        message: None,
                        repo_path: None,
                        branch: None,
                        last_sync: previous.last_sync.clone(),
                    },
                    Vec::new(),
                );
            }
            Detection::Error(message) => {
                log::warn!("settings sync: detection failed: {}", message);
                return (
                    SyncStatus {
                        state: SyncState::Error,
                        message: Some(message),
                        repo_path: None,
                        branch: None,
                        last_sync: previous.last_sync.clone(),
                    },
                    Vec::new(),
                );
            }
            Detection::Repo(repo) => repo,
        };

        let base = SyncStatus {
            state: SyncState::Synced,
            message: None,
            repo_path: Some(repo.top_level.to_string_lossy().into_owned()),
            branch: Some(repo.branch.clone()),
            last_sync: previous.last_sync.clone(),
        };

        match self.sync_repo(&runner, hooks, &repo).await {
            Ok(changed) => {
                self.set_paused(false);
                log::info!(
                    "settings sync: synced {} ({} files changed by rebase)",
                    repo.branch,
                    changed.len()
                );
                (
                    SyncStatus {
                        last_sync: Some(chrono::Utc::now().to_rfc3339()),
                        ..base
                    },
                    changed,
                )
            }
            Err(Stop::Conflict(message)) => {
                self.set_paused(true);
                log::warn!("settings sync: rebase conflict, paused: {}", message);
                (
                    SyncStatus {
                        state: SyncState::Conflict,
                        message: Some(message),
                        ..base
                    },
                    Vec::new(),
                )
            }
            Err(Stop::Error(message)) => {
                log::warn!("settings sync: failed: {}", message);
                (
                    SyncStatus {
                        state: SyncState::Error,
                        message: Some(message),
                        ..base
                    },
                    Vec::new(),
                )
            }
        }
    }

    async fn sync_repo<H: SyncHooks>(
        &self,
        runner: &GitRunner,
        hooks: &H,
        repo: &SyncRepo,
    ) -> Result<Vec<PathBuf>, Stop> {
        let top = repo.top_level.as_path();
        runner.run_network(top, &["fetch"]).await?;

        let changed = self.locked_phase(runner, hooks, repo).await?;

        let ahead = runner
            .run_ok(top, &["rev-list", "--count", "@{u}..HEAD"])
            .await?;
        if ahead.stdout.parse::<u64>().unwrap_or(0) > 0 {
            runner.run_network(top, &["push"]).await?;
        }
        Ok(changed)
    }

    async fn locked_phase<H: SyncHooks>(
        &self,
        runner: &GitRunner,
        hooks: &H,
        repo: &SyncRepo,
    ) -> Result<Vec<PathBuf>, Stop> {
        let mut guard = hooks.lock_local().await;
        let changed = self.commit_and_rebase(runner, repo).await?;
        if !changed.is_empty() {
            hooks.reload(&mut guard, &changed).await;
        }
        Ok(changed)
    }

    async fn commit_and_rebase(
        &self,
        runner: &GitRunner,
        repo: &SyncRepo,
    ) -> Result<Vec<PathBuf>, Stop> {
        let top = repo.top_level.as_path();

        let mut add = vec![OsString::from("add"), "-A".into(), "--".into()];
        add.extend(repo.stage_paths.iter().map(OsString::from));
        add.push(":(exclude)*.tmp".into());
        runner.run_ok(top, &add).await?;

        let mut diff = vec![OsString::from("diff"), "--cached".into(), "--quiet".into(), "--".into()];
        diff.extend(repo.stage_paths.iter().map(OsString::from));
        let staged = runner.run(top, &diff).await?;
        match staged.code {
            Some(0) => {}
            Some(1) => {
                let message = format!("chore(sync): update from {}", self.host_name(runner).await);
                let mut commit = vec![OsString::from("commit"), "-m".into(), message.into(), "--".into()];
                commit.extend(repo.stage_paths.iter().map(OsString::from));
                runner.run_ok(top, &commit).await?;
            }
            _ => {
                return Err(Stop::Error(format!("git diff failed: {}", staged.stderr)));
            }
        }

        let old_head = runner.run_ok(top, &["rev-parse", "HEAD"]).await?.stdout;
        let rebase = runner.run(top, &["rebase", "--autostash", "@{u}"]).await?;
        if !rebase.success {
            if rebase_in_progress(runner, top).await {
                if let Err(e) = runner.run_ok(top, &["rebase", "--abort"]).await {
                    log::warn!("settings sync: rebase --abort failed: {}", e);
                }
                return Err(Stop::Conflict(format!(
                    "rebase conflict, resolve manually in {}: {}",
                    top.display(),
                    rebase.stderr
                )));
            }
            return Err(Stop::Error(format!("git rebase failed: {}", rebase.stderr)));
        }

        let mut names = vec![
            OsString::from("diff"),
            "--name-only".into(),
            "-z".into(),
            old_head.into(),
            "HEAD".into(),
            "--".into(),
        ];
        names.extend(repo.stage_paths.iter().map(OsString::from));
        let diff = runner.run_ok(top, &names).await?;
        Ok(diff
            .stdout
            .split('\0')
            .filter(|name| !name.is_empty())
            .map(|name| top.join(name))
            .collect())
    }

    async fn host_name(&self, runner: &GitRunner) -> String {
        if let Some(host) = &self.configured_host {
            return host.clone();
        }
        self.host
            .get_or_init(|| async {
                runner
                    .hostname()
                    .await
                    .unwrap_or_else(|| UNKNOWN_HOST.to_string())
            })
            .await
            .clone()
    }
}

async fn rebase_in_progress(runner: &GitRunner, top: &Path) -> bool {
    for name in ["rebase-merge", "rebase-apply"] {
        if let Ok(output) = runner.run_ok(top, &["rev-parse", "--git-path", name]).await {
            if top.join(&output.stdout).exists() {
                return true;
            }
        }
    }
    false
}
