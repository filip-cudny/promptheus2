mod parser;

use std::path::{Path, PathBuf};

use log::{info, warn};
use rusqlite::{Connection, OptionalExtension};

use crate::models::skill::Skill;
use crate::services::database::sha256_hex;

use parser::parse_skill_file;

#[derive(Debug, thiserror::Error)]
pub enum SkillError {
    #[error("Skill not found: {0}")]
    NotFound(String),

    #[error("Invalid skill file '{0}': {1}")]
    InvalidFile(String, String),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("YAML parse error in '{0}': {1}")]
    YamlParse(String, String),

    #[error("SQLite error: {0}")]
    Sqlite(#[from] rusqlite::Error),
}

pub struct SkillService {
    skills: Vec<Skill>,
    skills_dir: PathBuf,
}

impl SkillService {
    pub fn load(
        skills_dir: &Path,
        resource_dir: Option<&Path>,
        order: &[String],
    ) -> Result<Self, SkillError> {
        if !skills_dir.exists() {
            Self::initialize_defaults(skills_dir, resource_dir)?;
        }

        let mut service = Self {
            skills: Vec::new(),
            skills_dir: skills_dir.to_path_buf(),
        };

        service.scan_and_load(order)?;
        Ok(service)
    }

    fn initialize_defaults(
        skills_dir: &Path,
        resource_dir: Option<&Path>,
    ) -> Result<(), SkillError> {
        std::fs::create_dir_all(skills_dir)?;
        info!("initializing default skills in {}", skills_dir.display());

        if let Some(res_dir) = resource_dir {
            let default_skills = res_dir.join("resources/skills");
            if default_skills.exists() {
                copy_skill_dirs(&default_skills, skills_dir)?;
                return Ok(());
            }
        }

        write_bundled_defaults(skills_dir)?;
        Ok(())
    }

    fn scan_and_load(&mut self, order: &[String]) -> Result<(), SkillError> {
        self.skills.clear();

        let entries = std::fs::read_dir(&self.skills_dir)?;
        let mut loaded = Vec::new();

        for entry in entries {
            let entry = entry?;
            let path = entry.path();

            if !path.is_dir() {
                continue;
            }

            let skill_file = path.join("SKILL.md");
            if !skill_file.exists() {
                continue;
            }

            let dir_name = entry.file_name().to_string_lossy().to_string();

            match parse_skill_file(&skill_file, &dir_name) {
                Ok(skill) => loaded.push(skill),
                Err(e) => warn!("skipping invalid skill {}: {e}", path.display()),
            }
        }

        if order.is_empty() {
            loaded.sort_by(|a, b| a.display_name.cmp(&b.display_name));
        } else {
            loaded.sort_by(|a, b| {
                let pos_a = order.iter().position(|n| n == &a.name).unwrap_or(usize::MAX);
                let pos_b = order.iter().position(|n| n == &b.name).unwrap_or(usize::MAX);
                pos_a.cmp(&pos_b)
            });
        }

        self.skills = loaded;
        Ok(())
    }

    pub fn reload(&mut self, order: &[String]) -> Result<(), SkillError> {
        self.scan_and_load(order)
    }

    pub fn list_skills(&self) -> &[Skill] {
        &self.skills
    }

    pub fn get_skill(&self, name: &str) -> Option<&Skill> {
        self.skills.iter().find(|s| s.name == name)
    }

    pub fn skills_dir(&self) -> &Path {
        &self.skills_dir
    }

    pub fn sync_versions(&mut self, conn: &Connection) -> Result<(), SkillError> {
        let now = chrono::Local::now()
            .format("%Y-%m-%d %H:%M:%S")
            .to_string();

        for skill in self.skills.iter_mut() {
            let skill_id: i64 = match conn
                .query_row(
                    "SELECT id FROM skills WHERE name = ?1",
                    [&skill.name],
                    |row| row.get::<_, i64>(0),
                )
                .optional()?
            {
                Some(id) => {
                    conn.execute(
                        "UPDATE skills SET display_name = ?1, deleted_at = NULL WHERE id = ?2",
                        rusqlite::params![skill.display_name, id],
                    )?;
                    id
                }
                None => {
                    conn.execute(
                        "INSERT INTO skills (name, display_name, created_at) VALUES (?1, ?2, ?3)",
                        rusqlite::params![skill.name, skill.display_name, now],
                    )?;
                    conn.last_insert_rowid()
                }
            };

            let body_hash = sha256_hex(&skill.body);
            let version_id: i64 = match conn
                .query_row(
                    "SELECT id FROM skill_versions WHERE skill_id = ?1 AND body_hash = ?2",
                    rusqlite::params![skill_id, body_hash],
                    |row| row.get::<_, i64>(0),
                )
                .optional()?
            {
                Some(id) => id,
                None => {
                    conn.execute(
                        "INSERT INTO skill_versions (skill_id, body, body_hash, created_at) VALUES (?1, ?2, ?3, ?4)",
                        rusqlite::params![skill_id, skill.body, body_hash, now],
                    )?;
                    info!(
                        "registered new version of skill '{}' (id={})",
                        skill.name,
                        conn.last_insert_rowid()
                    );
                    conn.last_insert_rowid()
                }
            };

            skill.skill_version_id = Some(version_id);
        }

        let active_names: Vec<String> = self.skills.iter().map(|s| s.name.clone()).collect();
        if !active_names.is_empty() {
            let placeholders: Vec<String> =
                (1..=active_names.len()).map(|i| format!("?{i}")).collect();
            let sql = format!(
                "UPDATE skills SET deleted_at = ?{} WHERE deleted_at IS NULL AND name NOT IN ({})",
                active_names.len() + 1,
                placeholders.join(",")
            );
            let mut params: Vec<&dyn rusqlite::ToSql> = active_names
                .iter()
                .map(|s| s as &dyn rusqlite::ToSql)
                .collect();
            params.push(&now);
            conn.execute(&sql, params.as_slice())?;
        }

        Ok(())
    }

    pub fn lookup_version_body(
        conn: &Connection,
        skill_version_id: i64,
    ) -> Result<Option<String>, rusqlite::Error> {
        conn.query_row(
            "SELECT body FROM skill_versions WHERE id = ?1",
            [skill_version_id],
            |row| row.get::<_, String>(0),
        )
        .optional()
    }
}

fn copy_skill_dirs(src: &Path, dest: &Path) -> Result<(), SkillError> {
    for entry in std::fs::read_dir(src)? {
        let entry = entry?;
        let src_path = entry.path();
        if src_path.is_dir() {
            let skill_file = src_path.join("SKILL.md");
            if skill_file.exists() {
                let dest_dir = dest.join(entry.file_name());
                std::fs::create_dir_all(&dest_dir)?;
                std::fs::copy(&skill_file, dest_dir.join("SKILL.md"))?;
            }
        }
    }
    Ok(())
}

fn write_bundled_defaults(skills_dir: &Path) -> Result<(), SkillError> {
    let defaults = [
        (
            "prompt-refine",
            include_str!("../../../resources/skills/prompt-refine/SKILL.md"),
        ),
        (
            "prompt-execute",
            include_str!("../../../resources/skills/prompt-execute/SKILL.md"),
        ),
        (
            "translate-english",
            include_str!("../../../resources/skills/translate-english/SKILL.md"),
        ),
        (
            "translate-polish",
            include_str!("../../../resources/skills/translate-polish/SKILL.md"),
        ),
        (
            "process-with-context",
            include_str!("../../../resources/skills/process-with-context/SKILL.md"),
        ),
    ];

    for (name, content) in defaults {
        let skill_dir = skills_dir.join(name);
        std::fs::create_dir_all(&skill_dir)?;
        std::fs::write(skill_dir.join("SKILL.md"), content)?;
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn write_skill_dir(parent: &Path, dir_name: &str, name: &str, description: &str, body: &str) {
        let skill_dir = parent.join(dir_name);
        std::fs::create_dir_all(&skill_dir).unwrap();
        let content = format!("---\nname: {name}\ndescription: {description}\n---\n\n{body}\n");
        std::fs::write(skill_dir.join("SKILL.md"), content).unwrap();
    }

    fn write_skill_dir_with_display(
        parent: &Path,
        dir_name: &str,
        name: &str,
        display_name: &str,
        description: &str,
        body: &str,
    ) {
        let skill_dir = parent.join(dir_name);
        std::fs::create_dir_all(&skill_dir).unwrap();
        let content = format!(
            "---\nname: {name}\ndisplay_name: {display_name}\ndescription: {description}\n---\n\n{body}\n"
        );
        std::fs::write(skill_dir.join("SKILL.md"), content).unwrap();
    }

    #[test]
    fn load_skills_from_directory() {
        let dir = TempDir::new().unwrap();
        let skills_dir = dir.path().join("skills");
        std::fs::create_dir(&skills_dir).unwrap();

        write_skill_dir(&skills_dir, "alpha", "alpha", "Alpha skill", "Alpha body");
        write_skill_dir(&skills_dir, "beta", "beta", "Beta skill", "Beta body");

        let service = SkillService::load(&skills_dir, None, &[]).unwrap();
        assert_eq!(service.list_skills().len(), 2);
        assert_eq!(service.list_skills()[0].display_name, "Alpha");
    }

    #[test]
    fn ordering_by_settings() {
        let dir = TempDir::new().unwrap();
        let skills_dir = dir.path().join("skills");
        std::fs::create_dir(&skills_dir).unwrap();

        write_skill_dir(&skills_dir, "alpha", "alpha", "desc", "body");
        write_skill_dir(&skills_dir, "beta", "beta", "desc", "body");
        write_skill_dir(&skills_dir, "gamma", "gamma", "desc", "body");

        let order = vec!["gamma".into(), "alpha".into(), "beta".into()];
        let service = SkillService::load(&skills_dir, None, &order).unwrap();
        let names: Vec<&str> = service.list_skills().iter().map(|s| s.name.as_str()).collect();
        assert_eq!(names, vec!["gamma", "alpha", "beta"]);
    }

    #[test]
    fn get_skill_by_name() {
        let dir = TempDir::new().unwrap();
        let skills_dir = dir.path().join("skills");
        std::fs::create_dir(&skills_dir).unwrap();

        write_skill_dir(&skills_dir, "test-skill", "test-skill", "desc", "body");

        let service = SkillService::load(&skills_dir, None, &[]).unwrap();
        assert!(service.get_skill("test-skill").is_some());
        assert!(service.get_skill("nonexistent").is_none());
    }

    #[test]
    fn reload_picks_up_new_dirs() {
        let dir = TempDir::new().unwrap();
        let skills_dir = dir.path().join("skills");
        std::fs::create_dir(&skills_dir).unwrap();

        write_skill_dir(&skills_dir, "one", "one", "desc", "body");
        let mut service = SkillService::load(&skills_dir, None, &[]).unwrap();
        assert_eq!(service.list_skills().len(), 1);

        write_skill_dir(&skills_dir, "two", "two", "desc", "body");
        service.reload(&[]).unwrap();
        assert_eq!(service.list_skills().len(), 2);
    }

    #[test]
    fn skips_non_dir_entries() {
        let dir = TempDir::new().unwrap();
        let skills_dir = dir.path().join("skills");
        std::fs::create_dir(&skills_dir).unwrap();

        write_skill_dir(&skills_dir, "valid", "valid", "desc", "body");
        std::fs::write(skills_dir.join("readme.txt"), "not a skill").unwrap();

        let service = SkillService::load(&skills_dir, None, &[]).unwrap();
        assert_eq!(service.list_skills().len(), 1);
    }

    #[test]
    fn skips_dir_without_skill_md() {
        let dir = TempDir::new().unwrap();
        let skills_dir = dir.path().join("skills");
        std::fs::create_dir(&skills_dir).unwrap();

        write_skill_dir(&skills_dir, "good", "good", "desc", "body");
        std::fs::create_dir(skills_dir.join("empty-dir")).unwrap();

        let service = SkillService::load(&skills_dir, None, &[]).unwrap();
        assert_eq!(service.list_skills().len(), 1);
        assert_eq!(service.list_skills()[0].name, "good");
    }

    #[test]
    fn explicit_display_name_overrides_derived() {
        let dir = TempDir::new().unwrap();
        let skills_dir = dir.path().join("skills");
        std::fs::create_dir(&skills_dir).unwrap();

        write_skill_dir_with_display(
            &skills_dir,
            "translate-en",
            "translate-en",
            "Translate - English",
            "desc",
            "body",
        );

        let service = SkillService::load(&skills_dir, None, &[]).unwrap();
        assert_eq!(service.list_skills()[0].display_name, "Translate - English");
    }
}
