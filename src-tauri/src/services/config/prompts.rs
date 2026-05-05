use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::services::env_resolve::resolve_env_refs;

use super::ConfigError;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PromptKind {
    System,
    AboutMe,
    Environment,
    InputFormat,
    TitleGeneration,
    SpeechToText,
}

impl PromptKind {
    pub const ALL: [PromptKind; 6] = [
        PromptKind::System,
        PromptKind::AboutMe,
        PromptKind::Environment,
        PromptKind::InputFormat,
        PromptKind::TitleGeneration,
        PromptKind::SpeechToText,
    ];

    pub fn default_path(&self) -> &'static str {
        match self {
            PromptKind::System => "prompts/base/system.md",
            PromptKind::AboutMe => "prompts/base/about_me.md",
            PromptKind::Environment => "prompts/base/environment.md",
            PromptKind::InputFormat => "prompts/base/input_format.md",
            PromptKind::TitleGeneration => "prompts/surfaces/title_generation.md",
            PromptKind::SpeechToText => "prompts/surfaces/speech_to_text.md",
        }
    }

    pub fn label(&self) -> &'static str {
        match self {
            PromptKind::System => "System prompt",
            PromptKind::AboutMe => "About me",
            PromptKind::Environment => "Environment section",
            PromptKind::InputFormat => "Input format guide",
            PromptKind::TitleGeneration => "Conversation title",
            PromptKind::SpeechToText => "Speech-to-text bias prompt",
        }
    }

    pub fn supports_env_placeholders(&self) -> bool {
        matches!(self, PromptKind::Environment)
    }

    pub fn default_content(&self) -> &'static str {
        match self {
            PromptKind::System => include_str!("../../../resources/prompts/base/system.md"),
            PromptKind::AboutMe => include_str!("../../../resources/prompts/base/about_me.md"),
            PromptKind::Environment => {
                include_str!("../../../resources/prompts/base/environment.md")
            }
            PromptKind::InputFormat => {
                include_str!("../../../resources/prompts/base/input_format.md")
            }
            PromptKind::TitleGeneration => {
                include_str!("../../../resources/prompts/surfaces/title_generation.md")
            }
            PromptKind::SpeechToText => {
                include_str!("../../../resources/prompts/surfaces/speech_to_text.md")
            }
        }
    }
}

pub struct PromptStore<'a> {
    config_dir: &'a Path,
}

impl<'a> PromptStore<'a> {
    pub fn new(config_dir: &'a Path) -> Self {
        Self { config_dir }
    }

    pub fn read(&self, raw_path: &str) -> Result<String, ConfigError> {
        let path = self.resolve(raw_path)?;
        Ok(std::fs::read_to_string(path).unwrap_or_default())
    }

    pub fn write(&self, raw_path: &str, content: &str) -> Result<(), ConfigError> {
        let path = self.resolve(raw_path)?;
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        write_atomic(&path, content)
    }

    pub fn ensure_default(&self, kind: PromptKind, raw_path: &str) -> Result<(), ConfigError> {
        let path = self.resolve(raw_path)?;
        if path.exists() {
            return Ok(());
        }
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        write_atomic(&path, kind.default_content())
    }

    pub fn resolve(&self, raw_path: &str) -> Result<PathBuf, ConfigError> {
        let resolved = resolve_env_refs(raw_path);
        let trimmed = resolved.trim();
        if trimmed.is_empty() {
            return Err(ConfigError::InvalidSettings(
                "prompt path is empty".to_string(),
            ));
        }

        let candidate = PathBuf::from(trimmed);
        if candidate.is_absolute() {
            return Err(ConfigError::InvalidSettings(format!(
                "prompt path must be relative to config dir: '{trimmed}'"
            )));
        }
        if candidate
            .components()
            .any(|c| matches!(c, std::path::Component::ParentDir))
        {
            return Err(ConfigError::InvalidSettings(format!(
                "prompt path must not contain '..': '{trimmed}'"
            )));
        }
        let ext_ok = candidate
            .extension()
            .and_then(|e| e.to_str())
            .map(|e| e.eq_ignore_ascii_case("md") || e.eq_ignore_ascii_case("markdown"))
            .unwrap_or(false);
        if !ext_ok {
            return Err(ConfigError::InvalidSettings(format!(
                "prompt path must end with .md or .markdown: '{trimmed}'"
            )));
        }

        Ok(self.config_dir.join(candidate))
    }
}

fn write_atomic(path: &Path, content: &str) -> Result<(), ConfigError> {
    let parent = path.parent().ok_or_else(|| {
        ConfigError::InvalidSettings(format!("path has no parent: {}", path.display()))
    })?;
    let mut tmp = parent.to_path_buf();
    let file_name = path
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("prompt");
    tmp.push(format!(".{file_name}.tmp"));
    std::fs::write(&tmp, content)?;
    std::fs::rename(&tmp, path)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn resolve_rejects_absolute_path() {
        let dir = TempDir::new().unwrap();
        let store = PromptStore::new(dir.path());
        let err = store.resolve("/etc/passwd").unwrap_err();
        assert!(err.to_string().contains("must be relative"));
    }

    #[test]
    fn resolve_rejects_parent_dir_traversal() {
        let dir = TempDir::new().unwrap();
        let store = PromptStore::new(dir.path());
        let err = store.resolve("../escape.md").unwrap_err();
        assert!(err.to_string().contains("'..'"));
    }

    #[test]
    fn resolve_rejects_non_markdown_extension() {
        let dir = TempDir::new().unwrap();
        let store = PromptStore::new(dir.path());
        let err = store.resolve("prompts/base/system.txt").unwrap_err();
        assert!(err.to_string().contains(".md or .markdown"));
    }

    #[test]
    fn resolve_rejects_empty_path() {
        let dir = TempDir::new().unwrap();
        let store = PromptStore::new(dir.path());
        let err = store.resolve("   ").unwrap_err();
        assert!(err.to_string().contains("empty"));
    }

    #[test]
    fn resolve_accepts_markdown_path() {
        let dir = TempDir::new().unwrap();
        let store = PromptStore::new(dir.path());
        let resolved = store.resolve("prompts/base/system.md").unwrap();
        assert_eq!(resolved, dir.path().join("prompts/base/system.md"));
    }

    #[test]
    fn resolve_resolves_env_refs_in_path() {
        std::env::set_var("TEST_PROMPT_NAME", "custom_prompt");
        let dir = TempDir::new().unwrap();
        let store = PromptStore::new(dir.path());
        let resolved = store
            .resolve("prompts/base/${TEST_PROMPT_NAME}.md")
            .unwrap();
        assert_eq!(
            resolved,
            dir.path().join("prompts/base/custom_prompt.md")
        );
        std::env::remove_var("TEST_PROMPT_NAME");
    }

    #[test]
    fn ensure_default_writes_when_missing_and_skips_when_present() {
        let dir = TempDir::new().unwrap();
        let store = PromptStore::new(dir.path());
        store
            .ensure_default(PromptKind::System, "prompts/base/system.md")
            .unwrap();
        let path = dir.path().join("prompts/base/system.md");
        assert!(path.exists());
        let first = std::fs::read_to_string(&path).unwrap();
        assert_eq!(first, PromptKind::System.default_content());

        std::fs::write(&path, "user override").unwrap();
        store
            .ensure_default(PromptKind::System, "prompts/base/system.md")
            .unwrap();
        let after = std::fs::read_to_string(&path).unwrap();
        assert_eq!(after, "user override");
    }

    #[test]
    fn write_then_read_roundtrips() {
        let dir = TempDir::new().unwrap();
        let store = PromptStore::new(dir.path());
        store
            .write("prompts/base/system.md", "hello world")
            .unwrap();
        let content = store.read("prompts/base/system.md").unwrap();
        assert_eq!(content, "hello world");
    }

    #[test]
    fn read_missing_file_returns_empty_string() {
        let dir = TempDir::new().unwrap();
        let store = PromptStore::new(dir.path());
        let content = store.read("prompts/base/system.md").unwrap();
        assert_eq!(content, "");
    }

    #[test]
    fn all_kinds_have_distinct_default_paths() {
        let mut paths = std::collections::HashSet::new();
        for kind in PromptKind::ALL {
            assert!(paths.insert(kind.default_path()), "duplicate: {kind:?}");
        }
    }

    #[test]
    fn only_environment_supports_placeholders() {
        for kind in PromptKind::ALL {
            let supports = kind.supports_env_placeholders();
            assert_eq!(supports, matches!(kind, PromptKind::Environment));
        }
    }
}
