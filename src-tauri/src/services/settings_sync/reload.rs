use std::path::{Path, PathBuf};

use rusqlite::Connection;

use crate::services::config::{ConfigError, ConfigService};
use crate::services::skill::{SkillError, SkillService};

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Reloaded {
    pub settings: bool,
    pub skills: bool,
    pub change_log: bool,
}

impl Reloaded {
    pub fn merge(&mut self, other: Reloaded) {
        self.settings |= other.settings;
        self.skills |= other.skills;
        self.change_log |= other.change_log;
    }
}

#[derive(Debug, thiserror::Error)]
pub enum ReloadError {
    #[error(transparent)]
    Config(#[from] ConfigError),
    #[error(transparent)]
    Skill(#[from] SkillError),
}

fn canonical_or_raw(path: &Path) -> PathBuf {
    std::fs::canonicalize(path).unwrap_or_else(|_| path.to_path_buf())
}

pub fn reload_changed(
    changed: &[PathBuf],
    settings_target: &Path,
    skills_target: &Path,
    change_log_dir: &Path,
    config: &mut ConfigService,
    skills: &mut SkillService,
    conn: &Connection,
) -> Result<Reloaded, ReloadError> {
    let settings_target = canonical_or_raw(settings_target);
    let skills_target = canonical_or_raw(skills_target);
    let change_log_dir = canonical_or_raw(change_log_dir);
    let changed: Vec<PathBuf> = changed.iter().map(|p| canonical_or_raw(p)).collect();

    let settings = changed.iter().any(|p| *p == settings_target);
    let skills_changed = changed.iter().any(|p| p.starts_with(&skills_target));

    let mut reloaded = Reloaded {
        change_log: changed.iter().any(|p| p.starts_with(&change_log_dir)),
        ..Reloaded::default()
    };
    if settings {
        config.reload()?;
        reloaded.settings = true;
    }
    if settings || skills_changed {
        skills.reload(&config.settings().skills_order.clone())?;
        skills.prune_missing_skills(conn)?;
        reloaded.skills = true;
    }
    Ok(reloaded)
}

pub fn reload_if_changed(
    config: &mut ConfigService,
    skills: &mut SkillService,
    conn: &Connection,
) -> Result<Reloaded, ReloadError> {
    if !config.reload_if_changed()? {
        return Ok(Reloaded::default());
    }
    skills.reload(&config.settings().skills_order.clone())?;
    skills.prune_missing_skills(conn)?;
    Ok(Reloaded {
        settings: true,
        skills: true,
        ..Reloaded::default()
    })
}
