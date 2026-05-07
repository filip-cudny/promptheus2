use std::path::{Component, Path, PathBuf};

use crate::services::env_resolve::resolve_env_refs;

use super::ConfigError;

/// Resolves a settings-supplied path to an absolute path beneath `config_dir`.
///
/// Expands `${VAR}` references (including the synthetic `${CONFIG_DIR}` exposed by
/// `loader::load_env`), then enforces three rules so the config dir stays self-contained:
/// the path must be non-empty, must be relative, and must not contain `..` components.
pub fn resolve_config_relative(raw_path: &str, config_dir: &Path) -> Result<PathBuf, ConfigError> {
    let resolved = resolve_env_refs(raw_path);
    let trimmed = resolved.trim();
    if trimmed.is_empty() {
        return Err(ConfigError::InvalidSettings("path is empty".to_string()));
    }

    let candidate = PathBuf::from(trimmed);
    if candidate.is_absolute() {
        return Err(ConfigError::InvalidSettings(format!(
            "path must be relative to config dir: '{trimmed}'"
        )));
    }
    if candidate
        .components()
        .any(|c| matches!(c, Component::ParentDir))
    {
        return Err(ConfigError::InvalidSettings(format!(
            "path must not contain '..': '{trimmed}'"
        )));
    }

    Ok(config_dir.join(candidate))
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn rejects_absolute_path() {
        let dir = TempDir::new().unwrap();
        let err = resolve_config_relative("/etc/passwd", dir.path()).unwrap_err();
        assert!(err.to_string().contains("must be relative"));
    }

    #[test]
    fn rejects_parent_dir_traversal() {
        let dir = TempDir::new().unwrap();
        let err = resolve_config_relative("../escape.txt", dir.path()).unwrap_err();
        assert!(err.to_string().contains("'..'"));
    }

    #[test]
    fn rejects_empty_path() {
        let dir = TempDir::new().unwrap();
        let err = resolve_config_relative("   ", dir.path()).unwrap_err();
        assert!(err.to_string().contains("empty"));
    }

    #[test]
    fn accepts_relative_path() {
        let dir = TempDir::new().unwrap();
        let resolved = resolve_config_relative("data/keyterms.txt", dir.path()).unwrap();
        assert_eq!(resolved, dir.path().join("data/keyterms.txt"));
    }

    #[test]
    fn expands_env_refs_before_validation() {
        std::env::set_var("TEST_PATH_LEAF", "leaf.txt");
        let dir = TempDir::new().unwrap();
        let resolved =
            resolve_config_relative("data/${TEST_PATH_LEAF}", dir.path()).unwrap();
        assert_eq!(resolved, dir.path().join("data/leaf.txt"));
        std::env::remove_var("TEST_PATH_LEAF");
    }

    #[test]
    fn rejects_path_that_expands_to_absolute() {
        let dir = TempDir::new().unwrap();
        let err = resolve_config_relative("${CONFIG_DIR}/data/x.txt", dir.path()).unwrap_err();
        assert!(err.to_string().contains("must be relative"));
    }
}
