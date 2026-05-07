use std::path::Path;

use super::prompts::{PromptKind, PromptStore};
use super::ConfigError;

pub fn load_env(config_dir: &Path) {
    let env_path = config_dir.join(".env");
    if env_path.exists() {
        let _ = dotenvy::from_path_override(&env_path);
    }
    let _ = dotenvy::dotenv_override();

    std::env::set_var("CONFIG_DIR", config_dir);
}

pub(super) fn initialize_defaults(
    config_dir: &Path,
    resource_dir: Option<&Path>,
) -> Result<(), ConfigError> {
    std::fs::create_dir_all(config_dir)?;
    log::info!("initializing default settings in {}", config_dir.display());

    let copied = if let Some(res_dir) = resource_dir {
        let default_settings = res_dir.join("resources/default_settings.json");
        if default_settings.exists() {
            std::fs::copy(&default_settings, config_dir.join("settings.json"))?;
            true
        } else {
            false
        }
    } else {
        false
    };

    if !copied {
        let fallback_json = include_str!("../../../resources/default_settings.json");
        std::fs::write(config_dir.join("settings.json"), fallback_json)?;
    }

    initialize_env(config_dir)?;
    initialize_prompt_defaults(config_dir)?;

    Ok(())
}

pub(super) fn initialize_prompt_defaults(config_dir: &Path) -> Result<(), ConfigError> {
    let store = PromptStore::new(config_dir);
    for kind in PromptKind::ALL {
        store.ensure_default(kind, kind.default_path())?;
    }
    Ok(())
}

fn initialize_env(config_dir: &Path) -> Result<(), ConfigError> {
    let env_path = config_dir.join(".env");
    if !env_path.exists() {
        std::fs::write(&env_path, "OPENAI_API_KEY=your_api_key_here\n")?;
    }
    Ok(())
}
