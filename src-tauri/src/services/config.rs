use std::path::{Path, PathBuf};

use crate::models::settings::{
    ApiKeySource, KeymapGroup, ModelConfig, NotificationSettings, PromptData, Settings,
    SpeechToTextModel,
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

        let content = std::fs::read_to_string(&settings_path)?;
        let mut settings: Settings = serde_json::from_str(&content)?;

        migrate_model_params(&mut settings);
        load_api_keys(&mut settings);

        if settings.default_model.is_none() && !settings.models.is_empty() {
            settings.default_model = Some(settings.models[0].id.clone());
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
        let mut settings = self.settings.clone();

        for model in &mut settings.models {
            if model.api_key_source != ApiKeySource::Direct {
                model.api_key = None;
            }
        }

        if let Some(ref mut speech) = settings.speech_to_text_model {
            speech.api_key = None;
        }

        let json = serde_json::to_string_pretty(&settings)?;
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
        load_api_keys(&mut settings);

        if settings.default_model.is_none() && !settings.models.is_empty() {
            settings.default_model = Some(settings.models[0].id.clone());
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
            "number_input_debounce_ms" => {
                if let Some(v) = value.as_u64() {
                    self.settings.number_input_debounce_ms = v as u32;
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

    pub fn add_prompt(&mut self, prompt: PromptData) {
        self.settings.prompts.push(prompt);
    }

    pub fn update_prompt(&mut self, prompt_id: &str, prompt: PromptData) {
        if let Some(existing) = self.settings.prompts.iter_mut().find(|p| p.id == prompt_id) {
            *existing = prompt;
        }
    }

    pub fn delete_prompt(&mut self, prompt_id: &str) {
        self.settings.prompts.retain(|p| p.id != prompt_id);
    }

    pub fn reorder_prompts(&mut self, prompt_ids: &[String]) {
        let mut reordered = Vec::with_capacity(prompt_ids.len());
        for id in prompt_ids {
            if let Some(pos) = self.settings.prompts.iter().position(|p| &p.id == id) {
                reordered.push(self.settings.prompts[pos].clone());
            }
        }
        self.settings.prompts = reordered;
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
}

pub fn load_env(config_dir: &Path) {
    let env_path = config_dir.join(".env");
    if env_path.exists() {
        let _ = dotenvy::from_path(&env_path);
    }
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

        match model.api_key_source {
            ApiKeySource::Env => {
                if model.api_key_env.as_deref().unwrap_or("").is_empty() {
                    return Err(ConfigError::InvalidSettings(format!(
                        "Model '{}' requires 'api_key_env' when api_key_source is 'env'",
                        display
                    )));
                }
            }
            ApiKeySource::Direct => {
                if model.api_key.as_deref().unwrap_or("").is_empty() {
                    return Err(ConfigError::InvalidSettings(format!(
                        "Model '{}' requires 'api_key' when api_key_source is 'direct'",
                        display
                    )));
                }
            }
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
        if speech.api_key_env.is_empty() {
            return Err(ConfigError::InvalidSettings(
                "speech_to_text_model field 'api_key_env' cannot be empty".to_string(),
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

pub fn load_api_keys(settings: &mut Settings) {
    for model in &mut settings.models {
        if model.api_key_source == ApiKeySource::Env {
            if let Some(ref env_var) = model.api_key_env {
                model.api_key = std::env::var(env_var).ok();
            }
        }
    }

    if let Some(ref mut speech) = settings.speech_to_text_model {
        speech.api_key = std::env::var(&speech.api_key_env).ok();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

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
        assert_eq!(service.settings().models[0].model, "gpt-4.1");
        assert_eq!(service.settings().prompts.len(), 5);
    }

    #[test]
    fn test_validate_missing_models() {
        let mut settings = Settings::default();
        settings.models.clear();
        let result = validate(&settings);
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("At least one model"));
    }

    #[test]
    fn test_validate_duplicate_model_ids() {
        let model = ModelConfig {
            id: "dup-id".to_string(),
            model: "test".to_string(),
            display_name: "Test".to_string(),
            api_key_source: ApiKeySource::Env,
            api_key_env: Some("TEST_KEY".to_string()),
            api_key: None,
            base_url: None,
            parameters: None,
        };
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
        assert_eq!(
            service.settings().models.len(),
            service2.settings().models.len()
        );
        assert_eq!(
            service.settings().models[0].id,
            service2.settings().models[0].id
        );


        service2.reload().expect("reload method");
        assert_eq!(
            service.settings().models[0].id,
            service2.settings().models[0].id
        );
    }

    #[test]
    fn test_api_key_sanitization_on_save() {
        let dir = setup_test_dir();
        let mut service = ConfigService::load(dir.path(), None).expect("load");


        service.settings_mut().models[0].api_key = Some("secret-key".to_string());


        service.add_model(ModelConfig {
            id: "direct-model".to_string(),
            model: "test".to_string(),
            display_name: "Direct Test".to_string(),
            api_key_source: ApiKeySource::Direct,
            api_key_env: None,
            api_key: Some("direct-secret".to_string()),
            base_url: None,
            parameters: None,
        });


        if let Some(ref mut speech) = service.settings_mut().speech_to_text_model {
            speech.api_key = Some("speech-secret".to_string());
        }

        service.save().expect("save");


        let content = fs::read_to_string(dir.path().join("settings.json")).unwrap();
        let saved: serde_json::Value = serde_json::from_str(&content).unwrap();


        let models = saved["models"].as_array().unwrap();
        assert!(models[0].get("api_key").map_or(true, |v| v.is_null()));


        assert_eq!(models[1]["api_key"].as_str().unwrap(), "direct-secret");


        let speech = &saved["speech_to_text_model"];
        assert!(speech.get("api_key").map_or(true, |v| v.is_null()));
    }

    #[test]
    fn test_load_api_keys_from_env() {
        let dir = setup_test_dir();


        std::env::set_var("TEST_CONFIG_API_KEY", "loaded-from-env");


        let content = fs::read_to_string(dir.path().join("settings.json")).unwrap();
        let mut json: serde_json::Value = serde_json::from_str(&content).unwrap();
        json["models"][0]["api_key_env"] = serde_json::Value::String("TEST_CONFIG_API_KEY".to_string());
        fs::write(
            dir.path().join("settings.json"),
            serde_json::to_string_pretty(&json).unwrap(),
        )
        .unwrap();

        let service = ConfigService::load(dir.path(), None).expect("load");
        assert_eq!(
            service.settings().models[0].api_key.as_deref(),
            Some("loaded-from-env")
        );

        std::env::remove_var("TEST_CONFIG_API_KEY");
    }

    #[test]
    fn test_migrate_model_params() {
        let mut settings = Settings {
            models: vec![ModelConfig {
                id: "1".to_string(),
                model: "test".to_string(),
                display_name: "Test".to_string(),
                api_key_source: ApiKeySource::Env,
                api_key_env: Some("KEY".to_string()),
                api_key: None,
                base_url: None,
                parameters: None,
            }],
            ..Default::default()
        };

        migrate_model_params(&mut settings);
        assert!(settings.models[0].parameters.is_some());
    }

    #[test]
    fn test_mutation_methods() {
        let dir = setup_test_dir();
        let mut service = ConfigService::load(dir.path(), None).expect("load");


        let new_model = ModelConfig {
            id: "new-model".to_string(),
            model: "gpt-5".to_string(),
            display_name: "GPT-5".to_string(),
            api_key_source: ApiKeySource::Env,
            api_key_env: Some("KEY".to_string()),
            api_key: None,
            base_url: None,
            parameters: None,
        };
        let initial_count = service.settings().models.len();
        service.add_model(new_model.clone());
        assert_eq!(service.settings().models.len(), initial_count + 1);


        let mut updated = new_model.clone();
        updated.display_name = "GPT-5 Updated".to_string();
        service.update_model("new-model", updated);
        assert_eq!(
            service
                .settings()
                .models
                .iter()
                .find(|m| m.id == "new-model")
                .unwrap()
                .display_name,
            "GPT-5 Updated"
        );


        let upsert_model = ModelConfig {
            id: "upsert-model".to_string(),
            model: "test".to_string(),
            display_name: "Upsert".to_string(),
            api_key_source: ApiKeySource::Env,
            api_key_env: Some("KEY".to_string()),
            api_key: None,
            base_url: None,
            parameters: None,
        };
        let count_before = service.settings().models.len();
        service.update_model("upsert-model", upsert_model);
        assert_eq!(service.settings().models.len(), count_before + 1);


        service.delete_model("upsert-model");
        assert!(service
            .settings()
            .models
            .iter()
            .all(|m| m.id != "upsert-model"));


        let prompt = PromptData {
            id: "test-prompt".to_string(),
            name: "Test".to_string(),
            description: None,
            messages: vec![],
        };
        let prompt_count = service.settings().prompts.len();
        service.add_prompt(prompt.clone());
        assert_eq!(service.settings().prompts.len(), prompt_count + 1);


        let mut updated_prompt = prompt;
        updated_prompt.name = "Updated Test".to_string();
        service.update_prompt("test-prompt", updated_prompt);
        assert_eq!(
            service
                .settings()
                .prompts
                .iter()
                .find(|p| p.id == "test-prompt")
                .unwrap()
                .name,
            "Updated Test"
        );


        service.delete_prompt("test-prompt");
        assert!(service
            .settings()
            .prompts
            .iter()
            .all(|p| p.id != "test-prompt"));


        let ids: Vec<String> = service.settings().prompts.iter().rev().map(|p| p.id.clone()).collect();
        let first_id_before = service.settings().prompts.last().unwrap().id.clone();
        service.reorder_prompts(&ids);
        assert_eq!(service.settings().prompts[0].id, first_id_before);


        service.update_setting("debug_mode", serde_json::Value::Bool(true));
        assert!(service.settings().debug_mode);
    }

    #[test]
    fn test_validate_invalid_base_url() {
        let settings = Settings {
            models: vec![ModelConfig {
                id: "1".to_string(),
                model: "test".to_string(),
                display_name: "Test".to_string(),
                api_key_source: ApiKeySource::Env,
                api_key_env: Some("KEY".to_string()),
                api_key: None,
                base_url: Some("ftp://invalid".to_string()),
                parameters: None,
            }],
            ..Default::default()
        };
        let result = validate(&settings);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("base_url"));
    }

    #[test]
    fn test_validate_debounce_out_of_range() {
        let settings = Settings {
            models: vec![ModelConfig {
                id: "1".to_string(),
                model: "test".to_string(),
                display_name: "Test".to_string(),
                api_key_source: ApiKeySource::Env,
                api_key_env: Some("KEY".to_string()),
                api_key: None,
                base_url: None,
                parameters: None,
            }],
            number_input_debounce_ms: 99999,
            ..Default::default()
        };
        let result = validate(&settings);
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("number_input_debounce_ms"));
    }

    #[test]
    fn test_validate_empty_required_fields() {
        let settings = Settings {
            models: vec![ModelConfig {
                id: "1".to_string(),
                model: "".to_string(),
                display_name: "Test".to_string(),
                api_key_source: ApiKeySource::Env,
                api_key_env: Some("KEY".to_string()),
                api_key: None,
                base_url: None,
                parameters: None,
            }],
            ..Default::default()
        };
        let result = validate(&settings);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("cannot be empty"));
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
        fs::write(
            resource_settings_dir.join("default_settings.json"),
            example,
        )
        .unwrap();

        let service =
            ConfigService::load(dir.path(), Some(resource_dir.path())).expect("should load");
        assert!(dir.path().join("settings.json").exists());
        assert_eq!(service.settings().models[0].model, "gpt-4.1");
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
