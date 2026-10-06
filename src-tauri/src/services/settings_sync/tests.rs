use std::collections::HashMap;
use std::os::unix::fs::{symlink, PermissionsExt};
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use async_trait::async_trait;
use tempfile::TempDir;
use tokio::sync::Notify;

use super::*;

const TIMEOUT: Duration = Duration::from_secs(20);

struct Fixture {
    tmp: TempDir,
    env: HashMap<String, String>,
}

impl Fixture {
    fn new() -> Self {
        let tmp = TempDir::new().unwrap();
        let gitconfig = tmp.path().join("gitconfig");
        std::fs::write(
            &gitconfig,
            "[user]\n\tname = Test\n\temail = test@example.com\n[init]\n\tdefaultBranch = main\n",
        )
        .unwrap();
        let env = HashMap::from([
            (
                "GIT_CONFIG_GLOBAL".to_string(),
                gitconfig.to_string_lossy().into_owned(),
            ),
            ("GIT_CONFIG_NOSYSTEM".to_string(), "1".to_string()),
        ]);
        let fixture = Self { tmp, env };
        fixture.build_repos();
        fixture
    }

    fn build_repos(&self) {
        let root = self.tmp.path();
        self.git(root, &["init", "--bare", "origin.git"]);
        self.git(root, &["clone", "origin.git", "a"]);
        let a = self.a();
        write(&a.join("tauri-app/settings.json"), "line1\nline2\n");
        write(&a.join("tauri-app/prompts/p.md"), "prompt\n");
        write(&a.join("tauri-app/skills/s/SKILL.md"), "skill\n");
        write(&a.join("tauri-app/other.txt"), "other\n");
        self.git(&a, &["add", "-A"]);
        self.git(&a, &["commit", "-m", "init"]);
        self.git(&a, &["push", "-u", "origin", "main"]);
        self.git(root, &["clone", "origin.git", "b"]);
    }

    fn origin(&self) -> PathBuf {
        self.tmp.path().join("origin.git")
    }

    fn a(&self) -> PathBuf {
        self.tmp.path().join("a")
    }

    fn b(&self) -> PathBuf {
        self.tmp.path().join("b")
    }

    fn git(&self, dir: &Path, args: &[&str]) -> String {
        let output = Command::new("git")
            .args(args)
            .current_dir(dir)
            .envs(&self.env)
            .output()
            .unwrap();
        assert!(
            output.status.success(),
            "git {:?} failed: {}",
            args,
            String::from_utf8_lossy(&output.stderr)
        );
        String::from_utf8_lossy(&output.stdout).trim().to_string()
    }

    fn config_dir_for(&self, clone: &Path, name: &str) -> PathBuf {
        let config_dir = self.tmp.path().join(name);
        std::fs::create_dir_all(&config_dir).unwrap();
        for entry in ["settings.json", "prompts", "skills"] {
            symlink(clone.join("tauri-app").join(entry), config_dir.join(entry)).unwrap();
        }
        config_dir
    }

    fn engine(&self, config_dir: PathBuf, env: HashMap<String, String>, timeout: Duration) -> SettingsSync {
        SettingsSync::new(
            config_dir,
            Arc::new(move || env.clone()),
            Some("testhost".to_string()),
            Some(timeout),
        )
    }

    fn engine_a(&self) -> SettingsSync {
        self.engine(self.config_dir_for(&self.a(), "config-a"), self.env.clone(), TIMEOUT)
    }

    fn runner(&self) -> GitRunner {
        GitRunner::new(self.env.clone(), TIMEOUT)
    }
}

fn write(path: &Path, content: &str) {
    std::fs::create_dir_all(path.parent().unwrap()).unwrap();
    std::fs::write(path, content).unwrap();
}

#[derive(Default)]
struct RecordingHooks {
    reloads: Mutex<Vec<Vec<PathBuf>>>,
    statuses: Mutex<Vec<SyncState>>,
    locks: Mutex<usize>,
}

#[async_trait]
impl SyncHooks for RecordingHooks {
    type Guard = ();

    async fn lock_local(&self) -> Self::Guard {
        *self.locks.lock().unwrap() += 1;
    }

    async fn reload(&self, _guard: &mut Self::Guard, changed: &[PathBuf]) {
        self.reloads.lock().unwrap().push(changed.to_vec());
    }

    fn status_changed(&self, status: &SyncStatus) {
        self.statuses.lock().unwrap().push(status.state);
    }
}

fn completed(result: SyncResult) -> CycleReport {
    match result {
        SyncResult::Completed(report) => report,
        other => panic!("expected a completed cycle, got {:?}", other),
    }
}

#[tokio::test]
async fn local_change_is_committed_and_pushed() {
    let f = Fixture::new();
    let a = f.a();
    write(&a.join("tauri-app/settings.json"), "line1 changed\nline2\n");
    write(&a.join("tauri-app/prompts/scratch.tmp"), "tmp\n");
    write(&a.join("tauri-app/extra.txt"), "staged elsewhere\n");
    f.git(&a, &["add", "tauri-app/extra.txt"]);

    let hooks = RecordingHooks::default();
    let report = completed(f.engine_a().sync(Trigger::Manual, &hooks).await);

    assert_eq!(report.status.state, SyncState::Synced);
    assert!(report.status.last_sync.is_some());
    assert_eq!(
        f.git(&f.origin(), &["log", "-1", "--format=%s", "main"]),
        "chore(sync): update from testhost"
    );
    let committed = f.git(&f.origin(), &["show", "--name-only", "--format=", "main"]);
    assert_eq!(committed, "tauri-app/settings.json");
    assert!(f
        .git(&a, &["status", "--porcelain"])
        .contains("A  tauri-app/extra.txt"));
}

#[tokio::test]
async fn modified_file_outside_targets_survives_sync() {
    let f = Fixture::new();
    write(&f.b().join("tauri-app/prompts/p.md"), "from b\n");
    f.git(&f.b(), &["commit", "-am", "b change"]);
    f.git(&f.b(), &["push"]);
    write(&f.a().join("tauri-app/other.txt"), "dirty\n");

    let hooks = RecordingHooks::default();
    let report = completed(f.engine_a().sync(Trigger::Manual, &hooks).await);

    assert_eq!(report.status.state, SyncState::Synced);
    assert_eq!(
        f.git(&f.a(), &["diff", "--name-only"]),
        "tauri-app/other.txt"
    );
}

#[tokio::test]
async fn remote_change_is_pulled_and_reported() {
    let f = Fixture::new();
    write(&f.b().join("tauri-app/prompts/p.md"), "from b\n");
    f.git(&f.b(), &["commit", "-am", "b change"]);
    f.git(&f.b(), &["push"]);

    let hooks = RecordingHooks::default();
    let report = completed(f.engine_a().sync(Trigger::Manual, &hooks).await);

    let expected = vec![f.a().canonicalize().unwrap().join("tauri-app/prompts/p.md")];
    assert_eq!(report.changed, expected);
    assert_eq!(*hooks.reloads.lock().unwrap(), vec![expected]);
    assert_eq!(
        std::fs::read_to_string(f.a().join("tauri-app/prompts/p.md")).unwrap(),
        "from b\n"
    );
}

#[tokio::test]
async fn conflict_aborts_rebase_and_pauses_periodic() {
    let f = Fixture::new();
    write(&f.b().join("tauri-app/settings.json"), "line1 from b\nline2\n");
    f.git(&f.b(), &["commit", "-am", "b change"]);
    f.git(&f.b(), &["push"]);
    write(&f.a().join("tauri-app/settings.json"), "line1 from a\nline2\n");

    let engine = f.engine_a();
    let hooks = RecordingHooks::default();
    let report = completed(engine.sync(Trigger::Startup, &hooks).await);

    assert_eq!(report.status.state, SyncState::Conflict);
    assert!(report.entered_new_state);
    for name in ["rebase-merge", "rebase-apply"] {
        let path = f.git(&f.a(), &["rev-parse", "--git-path", name]);
        assert!(!f.a().join(path).exists(), "{} still present", name);
    }
    assert_eq!(
        f.git(&f.a(), &["log", "-1", "--format=%s"]),
        "chore(sync): update from testhost"
    );

    let head = f.git(&f.a(), &["rev-parse", "HEAD"]);
    let reflog = f.git(&f.a(), &["reflog"]);
    assert_eq!(engine.sync(Trigger::Periodic, &hooks).await, SyncResult::Paused);
    assert_eq!(f.git(&f.a(), &["rev-parse", "HEAD"]), head);
    assert_eq!(f.git(&f.a(), &["reflog"]), reflog);
    assert_eq!(engine.status().state, SyncState::Conflict);

    let manual = completed(engine.sync(Trigger::Manual, &hooks).await);
    assert_eq!(manual.status.state, SyncState::Conflict);
    assert!(!manual.entered_new_state);
}

#[tokio::test]
async fn missing_origin_is_an_error() {
    let f = Fixture::new();
    f.git(&f.a(), &["remote", "remove", "origin"]);
    let hooks = RecordingHooks::default();

    let started = Instant::now();
    let report = completed(f.engine_a().sync(Trigger::Manual, &hooks).await);

    assert_eq!(report.status.state, SyncState::Error);
    assert!(started.elapsed() < Duration::from_secs(10));
}

#[tokio::test]
async fn origin_pointing_to_missing_path_is_an_error() {
    let f = Fixture::new();
    let missing = f.tmp.path().join("does-not-exist.git");
    f.git(&f.a(), &["remote", "set-url", "origin", missing.to_str().unwrap()]);
    let hooks = RecordingHooks::default();

    let started = Instant::now();
    let report = completed(f.engine_a().sync(Trigger::Manual, &hooks).await);

    assert_eq!(report.status.state, SyncState::Error);
    assert!(started.elapsed() < Duration::from_secs(10));
}

struct BlockingHooks {
    entered: Notify,
    release: Notify,
}

#[async_trait]
impl SyncHooks for BlockingHooks {
    type Guard = ();

    async fn lock_local(&self) -> Self::Guard {
        self.entered.notify_one();
        self.release.notified().await;
    }

    async fn reload(&self, _guard: &mut Self::Guard, _changed: &[PathBuf]) {}

    fn status_changed(&self, _status: &SyncStatus) {}
}

#[tokio::test]
async fn second_sync_while_running_returns_already_syncing() {
    let f = Fixture::new();
    let engine = Arc::new(f.engine_a());
    let hooks = Arc::new(BlockingHooks {
        entered: Notify::new(),
        release: Notify::new(),
    });

    let running = {
        let (engine, hooks) = (engine.clone(), hooks.clone());
        tokio::spawn(async move { engine.sync(Trigger::Manual, &*hooks).await })
    };
    hooks.entered.notified().await;

    assert_eq!(engine.status().state, SyncState::Syncing);
    assert_eq!(engine.sync(Trigger::Manual, &*hooks).await, SyncResult::AlreadySyncing);

    hooks.release.notify_one();
    let report = completed(running.await.unwrap());
    assert_eq!(report.status.state, SyncState::Synced);
}

#[tokio::test]
async fn ssh_remote_waiting_on_stdin_times_out() {
    for url in ["ssh://git@example.invalid/repo.git", "git@example.invalid:repo.git"] {
        let f = Fixture::new();
        let script = f.tmp.path().join("fake-ssh.sh");
        std::fs::write(&script, "#!/bin/sh\nread x\nsleep 30\n").unwrap();
        std::fs::set_permissions(&script, std::fs::Permissions::from_mode(0o755)).unwrap();
        f.git(&f.a(), &["remote", "set-url", "origin", url]);

        let mut env = f.env.clone();
        env.insert("GIT_SSH_COMMAND".to_string(), script.to_string_lossy().into_owned());
        let engine = f.engine(
            f.config_dir_for(&f.a(), "config-a"),
            env,
            Duration::from_secs(5),
        );
        let hooks = RecordingHooks::default();

        let started = Instant::now();
        let report = completed(engine.sync(Trigger::Manual, &hooks).await);

        assert_eq!(report.status.state, SyncState::Error, "{}", url);
        assert!(started.elapsed() < Duration::from_secs(10), "{}", url);
    }
}

#[tokio::test]
async fn detects_repo_from_symlinks() {
    let f = Fixture::new();
    let config_dir = f.config_dir_for(&f.a(), "config-a");

    let detection = detect(&f.runner(), &config_dir).await;

    assert_eq!(
        detection,
        Detection::Repo(SyncRepo {
            top_level: f.a().canonicalize().unwrap(),
            stage_paths: vec![
                PathBuf::from("tauri-app/settings.json"),
                PathBuf::from("tauri-app/prompts"),
                PathBuf::from("tauri-app/skills"),
            ],
            branch: "main".to_string(),
        })
    );
}

#[tokio::test]
async fn regular_files_are_off() {
    let f = Fixture::new();
    let config_dir = f.tmp.path().join("plain-config");
    write(&config_dir.join("settings.json"), "{}");
    std::fs::create_dir_all(config_dir.join("prompts")).unwrap();
    std::fs::create_dir_all(config_dir.join("skills")).unwrap();

    assert_eq!(detect(&f.runner(), &config_dir).await, Detection::Off);
}

#[tokio::test]
async fn targets_in_different_repos_are_an_error() {
    let f = Fixture::new();
    let config_dir = f.tmp.path().join("mixed-config");
    std::fs::create_dir_all(&config_dir).unwrap();
    symlink(f.a().join("tauri-app/settings.json"), config_dir.join("settings.json")).unwrap();
    symlink(f.b().join("tauri-app/prompts"), config_dir.join("prompts")).unwrap();

    assert!(matches!(
        detect(&f.runner(), &config_dir).await,
        Detection::Error(_)
    ));
}
