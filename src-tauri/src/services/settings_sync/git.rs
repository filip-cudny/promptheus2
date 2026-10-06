use std::collections::HashMap;
use std::ffi::OsStr;
use std::path::Path;
use std::process::Stdio;
use std::time::Duration;

use crate::services::shell_env::resolve_command;

#[derive(Debug, thiserror::Error)]
pub enum GitError {
    #[error("failed to start {program}: {reason}")]
    Spawn { program: String, reason: String },
    #[error("git {subcommand} timed out after {secs}s")]
    Timeout { subcommand: String, secs: u64 },
    #[error("git {subcommand} failed: {stderr}")]
    Failed { subcommand: String, stderr: String },
}

#[derive(Debug, Clone)]
pub struct GitOutput {
    pub success: bool,
    pub code: Option<i32>,
    pub stdout: String,
    pub stderr: String,
}

#[derive(Clone)]
pub struct GitRunner {
    env: HashMap<String, String>,
    timeout: Duration,
}

impl GitRunner {
    pub fn new(env: HashMap<String, String>, timeout: Duration) -> Self {
        Self { env, timeout }
    }

    pub async fn run<S: AsRef<OsStr>>(
        &self,
        dir: &Path,
        args: &[S],
    ) -> Result<GitOutput, GitError> {
        self.exec("git", Some(dir), &[], args).await
    }

    pub async fn run_ok<S: AsRef<OsStr>>(
        &self,
        dir: &Path,
        args: &[S],
    ) -> Result<GitOutput, GitError> {
        let output = self.run(dir, args).await?;
        Self::require_success(output, args)
    }

    pub async fn run_network<S: AsRef<OsStr>>(
        &self,
        dir: &Path,
        args: &[S],
    ) -> Result<GitOutput, GitError> {
        let config: &[&str] = if self.should_force_batch_ssh(dir).await {
            &["-c", "core.sshCommand=ssh -o BatchMode=yes"]
        } else {
            &[]
        };
        let output = self.exec("git", Some(dir), config, args).await?;
        Self::require_success(output, args)
    }

    pub async fn hostname(&self) -> Option<String> {
        let output = self.exec("hostname", None, &[], &["-s"]).await.ok()?;
        let name = output.stdout.trim().to_string();
        (output.success && !name.is_empty()).then_some(name)
    }

    fn require_success<S: AsRef<OsStr>>(
        output: GitOutput,
        args: &[S],
    ) -> Result<GitOutput, GitError> {
        if output.success {
            Ok(output)
        } else {
            Err(GitError::Failed {
                subcommand: subcommand_of(args),
                stderr: output.stderr,
            })
        }
    }

    async fn should_force_batch_ssh(&self, dir: &Path) -> bool {
        if self.env.contains_key("GIT_SSH_COMMAND") || std::env::var_os("GIT_SSH_COMMAND").is_some()
        {
            return false;
        }
        match self.run(dir, &["config", "--get", "core.sshCommand"]).await {
            Ok(output) => !output.success,
            Err(_) => false,
        }
    }

    async fn exec<S: AsRef<OsStr>>(
        &self,
        program: &str,
        dir: Option<&Path>,
        prefix: &[&str],
        args: &[S],
    ) -> Result<GitOutput, GitError> {
        let subcommand = subcommand_of(args);
        let resolved = resolve_command(program, &self.env);
        let mut command = tokio::process::Command::new(&resolved);
        command.args(prefix).args(args);
        command.envs(&self.env);
        command.env("GIT_TERMINAL_PROMPT", "0");
        if let Some(dir) = dir {
            command.current_dir(dir);
        }
        command
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .kill_on_drop(true);

        let child = command.spawn().map_err(|e| GitError::Spawn {
            program: program.to_string(),
            reason: e.to_string(),
        })?;

        let output = match tokio::time::timeout(self.timeout, child.wait_with_output()).await {
            Ok(Ok(output)) => output,
            Ok(Err(e)) => {
                return Err(GitError::Spawn {
                    program: program.to_string(),
                    reason: e.to_string(),
                })
            }
            Err(_) => {
                log::debug!("{} {} timed out", program, subcommand);
                return Err(GitError::Timeout {
                    subcommand,
                    secs: self.timeout.as_secs(),
                });
            }
        };

        log::debug!(
            "{} {} exited with {:?}",
            program,
            subcommand,
            output.status.code()
        );
        Ok(GitOutput {
            success: output.status.success(),
            code: output.status.code(),
            stdout: String::from_utf8_lossy(&output.stdout).trim().to_string(),
            stderr: String::from_utf8_lossy(&output.stderr).trim().to_string(),
        })
    }
}

fn subcommand_of<S: AsRef<OsStr>>(args: &[S]) -> String {
    args.first()
        .map(|a| a.as_ref().to_string_lossy().into_owned())
        .unwrap_or_default()
}
