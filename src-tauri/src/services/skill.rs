use std::path::{Path, PathBuf};

use log::{info, warn};

use crate::models::skill::{Skill, SkillFrontmatter};

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

            let dir_name = entry
                .file_name()
                .to_string_lossy()
                .to_string();

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
}

fn display_name_from_slug(slug: &str) -> String {
    slug.split('-')
        .map(|word| {
            let mut chars = word.chars();
            match chars.next() {
                None => String::new(),
                Some(c) => c.to_uppercase().to_string() + chars.as_str(),
            }
        })
        .collect::<Vec<_>>()
        .join(" ")
}

fn parse_skill_file(path: &Path, dir_name: &str) -> Result<Skill, SkillError> {
    let content = std::fs::read_to_string(path)?;

    let (frontmatter_str, body) = split_frontmatter(&content).ok_or_else(|| {
        SkillError::InvalidFile(dir_name.to_string(), "missing YAML frontmatter delimiters".into())
    })?;

    let fm: SkillFrontmatter = serde_yaml::from_str(frontmatter_str).map_err(|e| {
        SkillError::YamlParse(dir_name.to_string(), e.to_string())
    })?;

    let display_name = fm
        .display_name
        .unwrap_or_else(|| display_name_from_slug(&fm.name));

    Ok(Skill {
        name: fm.name,
        display_name,
        description: fm.description,
        model: fm.model,
        parameters: fm.parameters,
        body: body.trim().to_string(),
        file_path: path.to_path_buf(),
    })
}

fn split_frontmatter(content: &str) -> Option<(&str, &str)> {
    let trimmed = content.trim_start();
    if !trimmed.starts_with("---") {
        return None;
    }

    let after_first = &trimmed[3..];
    let end_idx = after_first.find("\n---")?;
    let frontmatter = &after_first[..end_idx].trim();
    let body = &after_first[end_idx + 4..];
    Some((frontmatter, body))
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
        ("prompt-refine", include_str!("../../resources/skills/prompt-refine/SKILL.md")),
        ("prompt-execute", include_str!("../../resources/skills/prompt-execute/SKILL.md")),
        ("translate-english", include_str!("../../resources/skills/translate-english/SKILL.md")),
        ("translate-polish", include_str!("../../resources/skills/translate-polish/SKILL.md")),
        ("process-with-context", include_str!("../../resources/skills/process-with-context/SKILL.md")),
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
        let content = format!(
            "---\nname: {name}\ndescription: {description}\n---\n\n{body}\n"
        );
        std::fs::write(skill_dir.join("SKILL.md"), content).unwrap();
    }

    fn write_skill_dir_with_display(parent: &Path, dir_name: &str, name: &str, display_name: &str, description: &str, body: &str) {
        let skill_dir = parent.join(dir_name);
        std::fs::create_dir_all(&skill_dir).unwrap();
        let content = format!(
            "---\nname: {name}\ndisplay_name: {display_name}\ndescription: {description}\n---\n\n{body}\n"
        );
        std::fs::write(skill_dir.join("SKILL.md"), content).unwrap();
    }

    #[test]
    fn parse_valid_skill_file() {
        let dir = TempDir::new().unwrap();
        let skill_dir = dir.path().join("test-skill");
        std::fs::create_dir(&skill_dir).unwrap();
        let path = skill_dir.join("SKILL.md");
        std::fs::write(&path, "---\nname: test-skill\ndescription: A test\n---\n\nDo something useful.\n\n<rules>\n- Rule 1\n</rules>\n").unwrap();

        let skill = parse_skill_file(&path, "test-skill").unwrap();
        assert_eq!(skill.name, "test-skill");
        assert_eq!(skill.display_name, "Test Skill");
        assert_eq!(skill.description, Some("A test".to_string()));
        assert!(skill.body.contains("Do something useful."));
        assert!(skill.body.contains("<rules>"));
    }

    #[test]
    fn parse_with_explicit_display_name() {
        let dir = TempDir::new().unwrap();
        let skill_dir = dir.path().join("translate-en");
        std::fs::create_dir(&skill_dir).unwrap();
        let path = skill_dir.join("SKILL.md");
        std::fs::write(&path, "---\nname: translate-en\ndisplay_name: Translate - English\ndescription: Translate\n---\n\nBody.\n").unwrap();

        let skill = parse_skill_file(&path, "translate-en").unwrap();
        assert_eq!(skill.display_name, "Translate - English");
    }

    #[test]
    fn parse_missing_frontmatter_fails() {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("SKILL.md");
        std::fs::write(&path, "No frontmatter here").unwrap();

        let result = parse_skill_file(&path, "bad");
        assert!(result.is_err());
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
    fn split_frontmatter_works() {
        let content = "---\nname: foo\n---\n\nBody here";
        let (fm, body) = split_frontmatter(content).unwrap();
        assert_eq!(fm, "name: foo");
        assert_eq!(body.trim(), "Body here");
    }

    #[test]
    fn split_frontmatter_no_closing() {
        let content = "---\nname: foo\nno closing delimiter";
        assert!(split_frontmatter(content).is_none());
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
    fn display_name_derived_from_slug() {
        assert_eq!(display_name_from_slug("translate-english"), "Translate English");
        assert_eq!(display_name_from_slug("prompt-refine"), "Prompt Refine");
        assert_eq!(display_name_from_slug("simple"), "Simple");
    }

    #[test]
    fn explicit_display_name_overrides_derived() {
        let dir = TempDir::new().unwrap();
        let skills_dir = dir.path().join("skills");
        std::fs::create_dir(&skills_dir).unwrap();

        write_skill_dir_with_display(&skills_dir, "translate-en", "translate-en", "Translate - English", "desc", "body");

        let service = SkillService::load(&skills_dir, None, &[]).unwrap();
        assert_eq!(service.list_skills()[0].display_name, "Translate - English");
    }
}
