use std::path::Path;

use crate::models::skill::{Skill, SkillFrontmatter};

use super::SkillError;

pub(super) fn display_name_from_slug(slug: &str) -> String {
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

pub(super) fn parse_skill_file(path: &Path, dir_name: &str) -> Result<Skill, SkillError> {
    let content = std::fs::read_to_string(path)?;

    let (frontmatter_str, body) = split_frontmatter(&content).ok_or_else(|| {
        SkillError::InvalidFile(
            dir_name.to_string(),
            "missing YAML frontmatter delimiters".into(),
        )
    })?;

    let fm: SkillFrontmatter = serde_yaml::from_str(frontmatter_str)
        .map_err(|e| SkillError::YamlParse(dir_name.to_string(), e.to_string()))?;

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
        skill_version_id: None,
    })
}

pub fn split_frontmatter(content: &str) -> Option<(&str, &str)> {
    let trimmed = content.trim_start();
    if !trimmed.starts_with("---") {
        return None;
    }

    let after_first = &trimmed[3..];
    let end_idx = after_first.find("\n---")?;
    let frontmatter = after_first[..end_idx].trim();
    let body = &after_first[end_idx + 4..];
    Some((frontmatter, body))
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

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
    fn split_frontmatter_with_trailing_body() {
        let content = "---\nname: foo\nkey: value\n---\nBody line one\nBody line two\n";
        let (fm, body) = split_frontmatter(content).unwrap();
        assert!(fm.contains("name: foo"));
        assert!(body.contains("Body line one"));
        assert!(body.contains("Body line two"));
    }

    #[test]
    fn display_name_derived_from_slug() {
        assert_eq!(display_name_from_slug("translate-english"), "Translate English");
        assert_eq!(display_name_from_slug("prompt-refine"), "Prompt Refine");
        assert_eq!(display_name_from_slug("simple"), "Simple");
    }

    #[test]
    fn display_name_handles_empty_segments() {
        assert_eq!(display_name_from_slug(""), "");
        assert_eq!(display_name_from_slug("a-b-c"), "A B C");
    }
}
