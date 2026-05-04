use std::path::Path;

use super::ConfigError;

pub fn load_env(config_dir: &Path) {
    let env_path = config_dir.join(".env");
    if env_path.exists() {
        let _ = dotenvy::from_path_override(&env_path);
    }
    let _ = dotenvy::dotenv_override();
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
    initialize_input_format_guide(config_dir)?;
    initialize_about_me(config_dir)?;
    initialize_environment_section(config_dir)?;

    Ok(())
}

pub(super) fn initialize_input_format_guide(config_dir: &Path) -> Result<(), ConfigError> {
    let guide_path = config_dir.join("input_format_guide.md");
    if !guide_path.exists() {
        let default_guide = include_str!("../../../resources/input_format_guide.md");
        std::fs::write(&guide_path, default_guide)?;
    }
    Ok(())
}

pub(super) fn initialize_about_me(config_dir: &Path) -> Result<(), ConfigError> {
    let about_me_path = config_dir.join("about_me.md");
    if !about_me_path.exists() {
        let default_about_me = include_str!("../../../resources/about_me.md");
        std::fs::write(&about_me_path, default_about_me)?;
    }
    Ok(())
}

pub(super) fn initialize_environment_section(config_dir: &Path) -> Result<(), ConfigError> {
    let path = config_dir.join("environment_section.md");
    if !path.exists() {
        let default = include_str!("../../../resources/environment_section.md");
        std::fs::write(&path, default)?;
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
