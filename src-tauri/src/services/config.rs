use std::path::{Path, PathBuf};

use crate::models::settings::{
    KeymapGroup, ModelConfig, NotificationSettings, Settings, SpeechToTextModel,
};

#[derive(Debug, thiserror::Error)]
pub enum ConfigError {
    #[error("Settings file not found: {0}")]
    FileNotFound(String),
    #[error("Invalid settings: {0}")]
    InvalidSettings(String),
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    #[error("JSON parse error: {0}")]
    JsonParse(#[from] serde_json::Error),
}

#[derive(Debug)]
pub struct ConfigService {
    settings: Settings,
    config_dir: PathBuf,
}

impl ConfigService {
    pub fn load(config_dir: &Path, resource_dir: Option<&Path>) -> Result<Self, ConfigError> {
        load_env(config_dir);

        let settings_path = config_dir.join("settings.json");
        if !settings_path.exists() {
            Self::initialize_defaults(config_dir, resource_dir)?;
        }

        let _ = Self::initialize_input_format_guide(config_dir);
        let _ = Self::initialize_about_me(config_dir);
        let _ = Self::initialize_environment_section(config_dir);

        let content = std::fs::read_to_string(&settings_path)?;
        let mut settings: Settings = serde_json::from_str(&content)?;

        migrate_model_params(&mut settings);
        migrate_legacy_env_fields(&mut settings);

        if settings.default_model.is_none() && !settings.models.is_empty() {
            settings.default_model = Some(settings.models[0].id.clone());
        }

        if settings.quick_action_default_model.is_none() {
            settings.quick_action_default_model = settings.default_model.clone();
        }

        validate(&settings)?;

        Ok(ConfigService {
            settings,
            config_dir: config_dir.to_path_buf(),
        })
    }

    fn initialize_defaults(
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
            let fallback_json = include_str!("../../resources/default_settings.json");
            std::fs::write(config_dir.join("settings.json"), fallback_json)?;
        }

        Self::initialize_env(config_dir)?;
        Self::initialize_input_format_guide(config_dir)?;
        Self::initialize_about_me(config_dir)?;
        Self::initialize_environment_section(config_dir)?;

        Ok(())
    }

    fn initialize_input_format_guide(config_dir: &Path) -> Result<(), ConfigError> {
        let guide_path = config_dir.join("input_format_guide.md");
        if !guide_path.exists() {
            let default_guide = include_str!("../../resources/input_format_guide.md");
            std::fs::write(&guide_path, default_guide)?;
        }
        Ok(())
    }

    fn initialize_about_me(config_dir: &Path) -> Result<(), ConfigError> {
        let about_me_path = config_dir.join("about_me.md");
        if !about_me_path.exists() {
            let default_about_me = include_str!("../../resources/about_me.md");
            std::fs::write(&about_me_path, default_about_me)?;
        }
        Ok(())
    }

    fn initialize_environment_section(config_dir: &Path) -> Result<(), ConfigError> {
        let path = config_dir.join("environment_section.md");
        if !path.exists() {
            let default = include_str!("../../resources/environment_section.md");
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

    pub fn save(&self) -> Result<(), ConfigError> {
        let json = serde_json::to_string_pretty(&self.settings)?;
        let settings_path = self.config_dir.join("settings.json");
        std::fs::write(&settings_path, json)?;
        Ok(())
    }

    pub fn reload(&mut self) -> Result<(), ConfigError> {
        let settings_path = self.config_dir.join("settings.json");
        if !settings_path.exists() {
            return Err(ConfigError::FileNotFound(
                settings_path.display().to_string(),
            ));
        }

        let content = std::fs::read_to_string(&settings_path)?;
        let mut settings: Settings = serde_json::from_str(&content)?;

        migrate_model_params(&mut settings);
        migrate_legacy_env_fields(&mut settings);

        if settings.default_model.is_none() && !settings.models.is_empty() {
            settings.default_model = Some(settings.models[0].id.clone());
        }

        if settings.quick_action_default_model.is_none() {
            settings.quick_action_default_model = settings.default_model.clone();
        }

        validate(&settings)?;
        self.settings = settings;
        log::debug!("settings reloaded");

        Ok(())
    }

    pub fn settings(&self) -> &Settings {
        &self.settings
    }

    pub fn settings_mut(&mut self) -> &mut Settings {
        &mut self.settings
    }

    pub fn update_setting(&mut self, key: &str, value: serde_json::Value) {
        match key {
            "show_tray_icon" => {
                if let Some(v) = value.as_bool() {
                    self.settings.show_tray_icon = v;
                }
            }
            "debug_mode" => {
                if let Some(v) = value.as_bool() {
                    self.settings.debug_mode = v;
                }
            }
            "code_theme" => {
                if let Some(v) = value.as_str() {
                    self.settings.code_theme = v.to_string();
                }
            }
            "default_model" => {
                self.settings.default_model = value.as_str().map(|s| s.to_string());
            }
            "quick_action_default_model" => {
                self.settings.quick_action_default_model = value.as_str().map(|s| s.to_string());
            }
            "number_input_debounce_ms" => {
                if let Some(v) = value.as_u64() {
                    self.settings.number_input_debounce_ms = v as u32;
                }
            }
            "conversation_title_model" => {
                if let Some(v) = value.as_str() {
                    self.settings.conversation_title_model = v.to_string();
                }
            }
            "conversation_title_prompt" => {
                if let Some(v) = value.as_str() {
                    self.settings.conversation_title_prompt = v.to_string();
                }
            }
            "selected_tools" => {
                if let Ok(v) = serde_json::from_value::<Vec<String>>(value) {
                    self.settings.selected_tools = v;
                }
            }
            _ => {}
        }
    }

    pub fn add_model(&mut self, config: ModelConfig) {
        self.settings.models.push(config);
    }

    pub fn update_model(&mut self, model_id: &str, config: ModelConfig) {
        if let Some(existing) = self.settings.models.iter_mut().find(|m| m.id == model_id) {
            *existing = config;
        } else {
            self.settings.models.push(config);
        }
    }

    pub fn delete_model(&mut self, model_id: &str) {
        self.settings.models.retain(|m| m.id != model_id);
    }

    pub fn update_notifications(&mut self, config: NotificationSettings) {
        self.settings.notifications = config;
    }

    pub fn update_speech_model(&mut self, config: SpeechToTextModel) {
        self.settings.speech_to_text_model = Some(config);
    }

    pub fn update_keymaps(&mut self, keymaps: Vec<KeymapGroup>) {
        self.settings.keymaps = keymaps;
    }

    pub fn update_menu_section_order(&mut self, order: Vec<String>) {
        self.settings.menu_section_order = order;
    }

    pub fn update_system_prompt(&mut self, prompt: String) {
        self.settings.system_prompt = prompt;
    }

    pub fn update_skills_order(&mut self, order: Vec<String>) {
        self.settings.skills_order = order;
    }

    pub fn input_format_guide(&self) -> String {
        let guide_path = self.config_dir.join("input_format_guide.md");
        std::fs::read_to_string(&guide_path).unwrap_or_default()
    }

    pub fn about_me(&self) -> String {
        let filename = self
            .settings
            .about_me
            .as_deref()
            .unwrap_or("about_me.md");
        let path = self.config_dir.join(filename);
        std::fs::read_to_string(&path).unwrap_or_default()
    }

    pub fn environment_section_template(&self) -> String {
        let filename = self
            .settings
            .environment_section
            .as_deref()
            .unwrap_or("environment_section.md");
        let path = self.config_dir.join(filename);
        std::fs::read_to_string(&path).unwrap_or_default()
    }

    pub fn config_dir(&self) -> &Path {
        &self.config_dir
    }
}

pub fn load_env(config_dir: &Path) {
    let env_path = config_dir.join(".env");
    if env_path.exists() {
        let _ = dotenvy::from_path_override(&env_path);
    }
    let _ = dotenvy::dotenv_override();
}

pub fn validate(settings: &Settings) -> Result<(), ConfigError> {
    if settings.models.is_empty() {
        return Err(ConfigError::InvalidSettings(
            "At least one model must be configured".to_string(),
        ));
    }

    let mut seen_ids = std::collections::HashSet::new();
    for model in &settings.models {
        if model.id.is_empty() {
            return Err(ConfigError::InvalidSettings(
                "Each model must have an 'id' field".to_string(),
            ));
        }

        if !seen_ids.insert(&model.id) {
            return Err(ConfigError::InvalidSettings(format!(
                "Duplicate model ID: '{}'",
                model.id
            )));
        }

        let display = &model.display_name;

        if model.model.is_empty() {
            return Err(ConfigError::InvalidSettings(format!(
                "Model '{}' field 'model' cannot be empty",
                display
            )));
        }

        if model.display_name.is_empty() {
            return Err(ConfigError::InvalidSettings(format!(
                "Model '{}' missing required field: display_name",
                model.id
            )));
        }

        if model.api_key.as_deref().unwrap_or("").is_empty() {
            return Err(ConfigError::InvalidSettings(format!(
                "Model '{}' requires 'api_key' (use \"${{ENV_VAR}}\" for env-based keys)",
                display
            )));
        }

        if let Some(ref url) = model.base_url {
            if !url.is_empty() && !url.starts_with("http://") && !url.starts_with("https://") {
                return Err(ConfigError::InvalidSettings(format!(
                    "Model '{}' base_url must start with http:// or https://",
                    display
                )));
            }
        }
    }

    if settings.number_input_debounce_ms > 10000 {
        return Err(ConfigError::InvalidSettings(
            "number_input_debounce_ms must be between 0 and 10000 milliseconds".to_string(),
        ));
    }

    if let Some(ref speech) = settings.speech_to_text_model {
        if speech.model.is_empty() {
            return Err(ConfigError::InvalidSettings(
                "speech_to_text_model field 'model' cannot be empty".to_string(),
            ));
        }
        if speech.display_name.is_empty() {
            return Err(ConfigError::InvalidSettings(
                "speech_to_text_model field 'display_name' cannot be empty".to_string(),
            ));
        }
        if speech.api_key.as_deref().unwrap_or("").is_empty() {
            return Err(ConfigError::InvalidSettings(
                "speech_to_text_model requires 'api_key' (use \"${ENV_VAR}\" for env-based keys)".to_string(),
            ));
        }
        if let Some(ref url) = speech.base_url {
            if !url.is_empty() && !url.starts_with("http://") && !url.starts_with("https://") {
                return Err(ConfigError::InvalidSettings(
                    "speech_to_text_model base_url must start with http:// or https://".to_string(),
                ));
            }
        }
    }

    Ok(())
}

pub fn migrate_model_params(settings: &mut Settings) {
    for model in &mut settings.models {
        if model.parameters.is_none() {
            model.parameters = Some(Default::default());
        }
    }
}

pub fn migrate_legacy_env_fields(settings: &mut Settings) {
    for model in &mut settings.models {
        if model.api_key.is_none() || model.api_key.as_deref() == Some("") {
            if let Some(ref env_var) = model.api_key_env {
                if !env_var.is_empty() {
                    log::info!("migrating model '{}' from api_key_env to ${{}} syntax", model.display_name);
                    model.api_key = Some(format!("${{{}}}", env_var));
                }
            }
        }
    }

    if let Some(ref mut speech) = settings.speech_to_text_model {
        if speech.api_key.is_none() || speech.api_key.as_deref() == Some("") {
            if let Some(ref env_var) = speech.api_key_env {
                if !env_var.is_empty() {
                    log::info!("migrating speech model from api_key_env to ${{}} syntax");
                    speech.api_key = Some(format!("${{{}}}", env_var));
                }
            }
        }
    }

    for (_name, server) in &mut settings.mcp_servers {
        if !server.env_inherit.is_empty() {
            log::info!("migrating MCP server env_inherit to ${{}} syntax");
            for (child_name, source_name) in std::mem::take(&mut server.env_inherit) {
                server.env.entry(child_name).or_insert_with(|| format!("${{{}}}", source_name));
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::settings::Provider;
    use std::fs;
    use tempfile::TempDir;

    fn test_model(id: &str, api_key: &str) -> ModelConfig {
        ModelConfig {
            id: id.to_string(),
            model: "test".to_string(),
            display_name: "Test".to_string(),
            provider: Provider::default(),
            api_key: Some(api_key.to_string()),
            base_url: None,
            parameters: None,
            context_window_size: None,
            api_mode: None,
            store: true,
            enabled_tools: vec![],
            api_key_source: None,
            api_key_env: None,
        }
    }

    fn setup_test_dir() -> TempDir {
        let dir = TempDir::new().unwrap();
        let example = include_str!("../../../../promptheus/settings_example/settings.json");
        fs::write(dir.path().join("settings.json"), example).unwrap();
        dir
    }

    #[test]
    fn test_load_example_settings() {
        let dir = setup_test_dir();
        let service = ConfigService::load(dir.path(), None).expect("should load example settings");
        assert!(!service.settings().models.is_empty());
        assert_eq!(service.settings().models[0].model, "gpt-5.4");
    }

    #[test]
    fn test_legacy_migration_converts_api_key_env() {
        let dir = setup_test_dir();
        let service = ConfigService::load(dir.path(), None).expect("load");
        assert_eq!(
            service.settings().models[0].api_key.as_deref(),
            Some("${OPENAI_API_KEY}")
        );
    }

    #[test]
    fn test_legacy_migration_converts_speech_api_key_env() {
        let dir = setup_test_dir();
        let service = ConfigService::load(dir.path(), None).expect("load");
        let speech = service.settings().speech_to_text_model.as_ref().unwrap();
        assert_eq!(speech.api_key.as_deref(), Some("${OPENAI_API_KEY}"));
    }

    #[test]
    fn test_validate_missing_models() {
        let mut settings = Settings::default();
        settings.models.clear();
        let result = validate(&settings);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("At least one model"));
    }

    #[test]
    fn test_validate_duplicate_model_ids() {
        let model = test_model("dup-id", "${KEY}");
        let settings = Settings {
            models: vec![model.clone(), model],
            ..Default::default()
        };
        let result = validate(&settings);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("Duplicate model ID"));
    }

    #[test]
    fn test_save_and_reload() {
        let dir = setup_test_dir();
        let service = ConfigService::load(dir.path(), None).expect("load");
        service.save().expect("save");

        let mut service2 = ConfigService::load(dir.path(), None).expect("reload");
        assert_eq!(service.settings().models.len(), service2.settings().models.len());
        assert_eq!(service.settings().models[0].id, service2.settings().models[0].id);

        service2.reload().expect("reload method");
        assert_eq!(service.settings().models[0].id, service2.settings().models[0].id);
    }

    #[test]
    fn test_env_ref_preserved_on_save() {
        let dir = setup_test_dir();
        let service = ConfigService::load(dir.path(), None).expect("load");
        service.save().expect("save");

        let content = fs::read_to_string(dir.path().join("settings.json")).unwrap();
        let saved: serde_json::Value = serde_json::from_str(&content).unwrap();
        let models = saved["models"].as_array().unwrap();
        assert_eq!(models[0]["api_key"].as_str().unwrap(), "${OPENAI_API_KEY}");

        let speech = &saved["speech_to_text_model"];
        assert_eq!(speech["api_key"].as_str().unwrap(), "${OPENAI_API_KEY}");
    }

    #[test]
    fn test_direct_api_key_preserved_on_save() {
        let dir = setup_test_dir();
        let mut service = ConfigService::load(dir.path(), None).expect("load");
        service.add_model(test_model("direct-model", "sk-direct-secret"));
        service.save().expect("save");

        let content = fs::read_to_string(dir.path().join("settings.json")).unwrap();
        let saved: serde_json::Value = serde_json::from_str(&content).unwrap();
        let models = saved["models"].as_array().unwrap();
        let direct = models.iter().find(|m| m["id"] == "direct-model").unwrap();
        assert_eq!(direct["api_key"].as_str().unwrap(), "sk-direct-secret");
    }

    #[test]
    fn test_legacy_api_key_env_migrated() {
        let dir = TempDir::new().unwrap();
        let json = r#"{
            "models": [{
                "id": "1", "model": "gpt-4", "display_name": "GPT-4",
                "api_key_source": "env", "api_key_env": "MY_KEY"
            }]
        }"#;
        fs::write(dir.path().join("settings.json"), json).unwrap();
        let service = ConfigService::load(dir.path(), None).expect("load");
        assert_eq!(service.settings().models[0].api_key.as_deref(), Some("${MY_KEY}"));
    }

    #[test]
    fn test_legacy_mcp_env_inherit_migrated() {
        let dir = TempDir::new().unwrap();
        let json = r#"{
            "models": [{"id": "1", "model": "t", "display_name": "T", "api_key": "${K}"}],
            "mcp_servers": {
                "srv": {
                    "command": "npx",
                    "env_inherit": {"CHILD_KEY": "PARENT_KEY"}
                }
            }
        }"#;
        fs::write(dir.path().join("settings.json"), json).unwrap();
        let service = ConfigService::load(dir.path(), None).expect("load");
        let srv = &service.settings().mcp_servers["srv"];
        assert_eq!(srv.env.get("CHILD_KEY").unwrap(), "${PARENT_KEY}");
        assert!(srv.env_inherit.is_empty());
    }

    #[test]
    fn test_migrate_model_params() {
        let mut settings = Settings {
            models: vec![test_model("1", "${KEY}")],
            ..Default::default()
        };
        settings.models[0].parameters = None;
        migrate_model_params(&mut settings);
        assert!(settings.models[0].parameters.is_some());
    }

    #[test]
    fn test_mutation_methods() {
        let dir = setup_test_dir();
        let mut service = ConfigService::load(dir.path(), None).expect("load");

        let new_model = test_model("new-model", "${KEY}");
        let initial_count = service.settings().models.len();
        service.add_model(new_model.clone());
        assert_eq!(service.settings().models.len(), initial_count + 1);

        let mut updated = new_model.clone();
        updated.display_name = "GPT-5 Updated".to_string();
        service.update_model("new-model", updated);
        assert_eq!(
            service.settings().models.iter().find(|m| m.id == "new-model").unwrap().display_name,
            "GPT-5 Updated"
        );

        let upsert = test_model("upsert-model", "${KEY}");
        let count_before = service.settings().models.len();
        service.update_model("upsert-model", upsert);
        assert_eq!(service.settings().models.len(), count_before + 1);

        service.delete_model("upsert-model");
        assert!(service.settings().models.iter().all(|m| m.id != "upsert-model"));

        service.update_setting("debug_mode", serde_json::Value::Bool(true));
        assert!(service.settings().debug_mode);
    }

    #[test]
    fn test_validate_invalid_base_url() {
        let mut model = test_model("1", "${KEY}");
        model.base_url = Some("ftp://invalid".to_string());
        let settings = Settings { models: vec![model], ..Default::default() };
        let result = validate(&settings);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("base_url"));
    }

    #[test]
    fn test_validate_debounce_out_of_range() {
        let settings = Settings {
            models: vec![test_model("1", "${KEY}")],
            number_input_debounce_ms: 99999,
            ..Default::default()
        };
        let result = validate(&settings);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("number_input_debounce_ms"));
    }

    #[test]
    fn test_validate_empty_required_fields() {
        let mut model = test_model("1", "${KEY}");
        model.model = "".to_string();
        let settings = Settings { models: vec![model], ..Default::default() };
        let result = validate(&settings);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("cannot be empty"));
    }

    #[test]
    fn test_validate_missing_api_key() {
        let mut model = test_model("1", "${KEY}");
        model.api_key = None;
        let settings = Settings { models: vec![model], ..Default::default() };
        let result = validate(&settings);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("api_key"));
    }

    #[test]
    fn test_first_run_creates_defaults_from_fallback() {
        let dir = TempDir::new().unwrap();
        let service = ConfigService::load(dir.path(), None).expect("should create defaults");
        assert!(dir.path().join("settings.json").exists());
        assert!(dir.path().join(".env").exists());
        assert!(!service.settings().models.is_empty());
    }

    #[test]
    fn test_first_run_copies_from_resource() {
        let dir = TempDir::new().unwrap();
        let resource_dir = TempDir::new().unwrap();
        let resource_settings_dir = resource_dir.path().join("resources");
        fs::create_dir_all(&resource_settings_dir).unwrap();
        let example = include_str!("../../../../promptheus/settings_example/settings.json");
        fs::write(resource_settings_dir.join("default_settings.json"), example).unwrap();

        let service = ConfigService::load(dir.path(), Some(resource_dir.path())).expect("should load");
        assert!(dir.path().join("settings.json").exists());
        assert_eq!(service.settings().models[0].model, "gpt-5.4");
    }

    #[test]
    fn test_first_run_does_not_overwrite_existing() {
        let dir = setup_test_dir();
        let original_content = fs::read_to_string(dir.path().join("settings.json")).unwrap();
        let _service = ConfigService::load(dir.path(), None).expect("should load existing");
        let after_content = fs::read_to_string(dir.path().join("settings.json")).unwrap();
        assert_eq!(original_content, after_content);
    }

    #[test]
    fn test_first_run_env_template_created() {
        let dir = TempDir::new().unwrap();
        let _service = ConfigService::load(dir.path(), None).expect("should create defaults");
        let env_content = fs::read_to_string(dir.path().join(".env")).unwrap();
        assert!(env_content.contains("OPENAI_API_KEY"));
    }

    #[test]
    fn test_existing_env_not_overwritten() {
        let dir = TempDir::new().unwrap();
        fs::create_dir_all(dir.path()).unwrap();
        fs::write(dir.path().join(".env"), "OPENAI_API_KEY=real_key\n").unwrap();
        let _service = ConfigService::load(dir.path(), None).expect("should create defaults");
        let env_content = fs::read_to_string(dir.path().join(".env")).unwrap();
        assert_eq!(env_content, "OPENAI_API_KEY=real_key\n");
    }
}
