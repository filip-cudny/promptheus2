use std::path::{Path, PathBuf};

use crate::models::settings::{
    KeymapGroup, ModelConfig, NotificationSettings, Settings, SpeechToTextConfig,
};
use crate::services::env_resolve::resolve_env_refs;

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
        let mut settings = parse_and_migrate(&content)?;

        migrate_model_params(&mut settings);
        ensure_surface_defaults(&mut settings);

        validate(&settings)?;

        let service = ConfigService {
            settings,
            config_dir: config_dir.to_path_buf(),
        };

        service.save()?;

        Ok(service)
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
        let mut settings = parse_and_migrate(&content)?;

        migrate_model_params(&mut settings);
        ensure_surface_defaults(&mut settings);

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
            "theme" => {
                if let Some(v) = value.as_str() {
                    self.settings.theme = v.to_string();
                }
            }
            "number_input_debounce_ms" => {
                if let Some(v) = value.as_u64() {
                    self.settings.number_input_debounce_ms = v as u32;
                }
            }
            _ => {}
        }
    }

    pub fn update_surface_model(&mut self, surface: SurfaceKind, model_id: Option<String>) {
        match surface {
            SurfaceKind::Chat => self.settings.surfaces.chat.generation.model_id = model_id,
            SurfaceKind::QuickActions => {
                self.settings.surfaces.quick_actions.generation.model_id = model_id
            }
            SurfaceKind::TitleGeneration => {
                self.settings.surfaces.title_generation.generation.model_id = model_id
            }
            SurfaceKind::SpeechToText => self.settings.surfaces.speech_to_text.model_id = model_id,
        }
    }

    pub fn update_surface_parameter(
        &mut self,
        surface: SurfaceKind,
        key: &str,
        value: serde_json::Value,
    ) {
        let params = match surface {
            SurfaceKind::Chat => &mut self.settings.surfaces.chat.generation.parameters,
            SurfaceKind::QuickActions => {
                &mut self.settings.surfaces.quick_actions.generation.parameters
            }
            SurfaceKind::TitleGeneration => {
                &mut self.settings.surfaces.title_generation.generation.parameters
            }
            SurfaceKind::SpeechToText => {
                log::warn!("update_surface_parameter: speech_to_text has no parameters field");
                return;
            }
        };

        let is_null = value.is_null();
        match key {
            "temperature" => params.temperature = if is_null { None } else { value.as_f64() },
            "max_tokens" => {
                params.max_tokens = if is_null {
                    None
                } else {
                    value.as_u64().map(|v| v as u32)
                }
            }
            "top_p" => params.top_p = if is_null { None } else { value.as_f64() },
            "frequency_penalty" => {
                params.frequency_penalty = if is_null { None } else { value.as_f64() }
            }
            "presence_penalty" => {
                params.presence_penalty = if is_null { None } else { value.as_f64() }
            }
            "reasoning_effort" => {
                params.reasoning_effort = if is_null {
                    None
                } else {
                    value.as_str().map(String::from)
                };
            }
            _ => {
                if is_null {
                    params.extra.remove(key);
                } else {
                    params.extra.insert(key.to_string(), value);
                }
            }
        }
    }

    pub fn update_surface_enabled_tools(&mut self, surface: SurfaceKind, tools: Vec<String>) {
        match surface {
            SurfaceKind::Chat => self.settings.surfaces.chat.generation.enabled_tools = tools,
            SurfaceKind::QuickActions => {
                self.settings.surfaces.quick_actions.generation.enabled_tools = tools
            }
            SurfaceKind::TitleGeneration => {
                self.settings.surfaces.title_generation.generation.enabled_tools = tools
            }
            SurfaceKind::SpeechToText => {
                log::warn!("update_surface_enabled_tools: speech_to_text has no tools");
            }
        }
    }

    pub fn update_speech_to_text(&mut self, config: SpeechToTextConfig) {
        self.settings.surfaces.speech_to_text = config;
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
        let s = &mut self.settings.surfaces;
        if s.chat.generation.model_id.as_deref() == Some(model_id) {
            s.chat.generation.model_id = None;
        }
        if s.quick_actions.generation.model_id.as_deref() == Some(model_id) {
            s.quick_actions.generation.model_id = None;
        }
        if s.title_generation.generation.model_id.as_deref() == Some(model_id) {
            s.title_generation.generation.model_id = None;
        }
        if s.speech_to_text.model_id.as_deref() == Some(model_id) {
            s.speech_to_text.model_id = None;
        }
    }

    pub fn update_notifications(&mut self, config: NotificationSettings) {
        self.settings.notifications = config;
    }

    pub fn resolve_stt_model(&self) -> Option<&ModelConfig> {
        let id = self.settings.surfaces.speech_to_text.model_id.as_deref()?;
        self.settings
            .models
            .iter()
            .find(|m| m.id == id && m.is_stt())
    }

    pub fn update_keymaps(&mut self, keymaps: Vec<KeymapGroup>) {
        self.settings.keymaps = keymaps;
    }

    pub fn update_menu_section_order(&mut self, order: Vec<String>) {
        self.settings.menu_section_order = order;
    }

    pub fn update_system_prompt(&mut self, prompt: String) {
        self.settings.prompt_base.system_prompt = prompt;
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
            .prompt_base
            .about_me
            .as_deref()
            .unwrap_or("about_me.md");
        let path = self.config_dir.join(filename);
        std::fs::read_to_string(&path).unwrap_or_default()
    }

    pub fn environment_section_template(&self) -> String {
        let filename = self
            .settings
            .prompt_base
            .environment_section
            .as_deref()
            .unwrap_or("environment_section.md");
        let path = self.config_dir.join(filename);
        std::fs::read_to_string(&path).unwrap_or_default()
    }

    pub fn stt_prompt(&self) -> Option<String> {
        let raw = self
            .settings
            .surfaces
            .speech_to_text
            .prompt
            .as_deref()
            .unwrap_or("stt_prompt.md");
        let filename = resolve_env_refs(raw);
        let path = self.config_dir.join(filename);
        let content = std::fs::read_to_string(&path).ok()?;
        let trimmed = content.trim();
        if trimmed.is_empty() { None } else { Some(trimmed.to_string()) }
    }

    pub fn stt_keyterms(&self) -> Vec<String> {
        let Some(raw) = self.settings.surfaces.speech_to_text.keyterms_file.as_deref() else {
            return Vec::new();
        };
        let filename = resolve_env_refs(raw);
        if filename.is_empty() {
            return Vec::new();
        }
        let path = self.config_dir.join(filename);
        let Ok(content) = std::fs::read_to_string(&path) else {
            return Vec::new();
        };
        content
            .lines()
            .map(str::trim)
            .filter(|line| !line.is_empty() && !line.starts_with('#'))
            .map(str::to_string)
            .collect()
    }

    pub fn config_dir(&self) -> &Path {
        &self.config_dir
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Deserialize, serde::Serialize)]
#[serde(rename_all = "snake_case")]
pub enum SurfaceKind {
    Chat,
    QuickActions,
    TitleGeneration,
    SpeechToText,
}

pub fn load_env(config_dir: &Path) {
    let env_path = config_dir.join(".env");
    if env_path.exists() {
        let _ = dotenvy::from_path_override(&env_path);
    }
    let _ = dotenvy::dotenv_override();
}

fn parse_and_migrate(content: &str) -> Result<Settings, ConfigError> {
    let raw: serde_json::Value = serde_json::from_str(content)?;
    let needs_migration = raw
        .as_object()
        .map(|obj| {
            !obj.contains_key("surfaces")
                || has_legacy_fields(obj)
                || has_chat_prompt_base_fields(obj)
        })
        .unwrap_or(false);

    if needs_migration {
        log::info!("migrating settings.json to new schema");
        let migrated = migrate_legacy_json(raw);
        let settings: Settings = serde_json::from_value(migrated)?;
        Ok(settings)
    } else {
        let settings: Settings = serde_json::from_value(raw)?;
        Ok(settings)
    }
}

fn has_legacy_fields(obj: &serde_json::Map<String, serde_json::Value>) -> bool {
    const LEGACY_KEYS: &[&str] = &[
        "default_model",
        "quick_action_default_model",
        "speech_to_text_model",
        "conversation_title_model",
        "conversation_title_prompt",
        "system_prompt",
        "about_me",
        "environment_section",
        "stt_prompt",
        "selected_tools",
    ];
    LEGACY_KEYS.iter().any(|k| obj.contains_key(*k))
}

fn has_chat_prompt_base_fields(obj: &serde_json::Map<String, serde_json::Value>) -> bool {
    let Some(chat) = obj
        .get("surfaces")
        .and_then(|v| v.as_object())
        .and_then(|s| s.get("chat"))
        .and_then(|c| c.as_object())
    else {
        return false;
    };
    chat.contains_key("system_prompt")
        || chat.contains_key("about_me")
        || chat.contains_key("environment_section")
}

fn migrate_legacy_json(mut raw: serde_json::Value) -> serde_json::Value {
    let Some(obj) = raw.as_object_mut() else {
        return raw;
    };

    let default_model = obj.remove("default_model");
    let quick_action_default_model = obj.remove("quick_action_default_model");
    let speech_to_text_model = obj.remove("speech_to_text_model");
    let conversation_title_model = obj.remove("conversation_title_model");
    let conversation_title_prompt = obj.remove("conversation_title_prompt");
    let mut system_prompt = obj.remove("system_prompt");
    let mut about_me = obj.remove("about_me");
    let mut environment_section = obj.remove("environment_section");
    let stt_prompt = obj.remove("stt_prompt");
    let selected_tools = obj.remove("selected_tools");

    let (stt_language, stt_keyterms_file, stt_no_verbatim, chat_reasoning_effort) =
        extract_model_level_fields(obj, speech_to_text_model.as_ref());

    let mut surfaces = obj
        .remove("surfaces")
        .and_then(|v| v.as_object().cloned())
        .unwrap_or_default();

    merge_into_surface(
        &mut surfaces,
        "chat",
        |chat| {
            let gen = ensure_generation(chat);
            if !gen.contains_key("model_id") {
                if let Some(v) = default_model.clone() {
                    gen.insert("model_id".to_string(), v);
                }
            }
            if !gen.contains_key("enabled_tools") {
                if let Some(v) = selected_tools.clone() {
                    gen.insert("enabled_tools".to_string(), v);
                }
            }
            if let Some(effort) = chat_reasoning_effort.clone() {
                let params = ensure_parameters(gen);
                if !params.contains_key("reasoning_effort") {
                    params.insert("reasoning_effort".to_string(), effort);
                }
            }
            if let Some(v) = chat.remove("system_prompt") {
                if system_prompt.is_none() {
                    system_prompt = Some(v);
                }
            }
            if let Some(v) = chat.remove("about_me") {
                if about_me.is_none() {
                    about_me = Some(v);
                }
            }
            if let Some(v) = chat.remove("environment_section") {
                if environment_section.is_none() {
                    environment_section = Some(v);
                }
            }
        },
    );

    merge_into_surface(
        &mut surfaces,
        "quick_actions",
        |qa| {
            let gen = ensure_generation(qa);
            if !gen.contains_key("model_id") {
                let v = quick_action_default_model
                    .clone()
                    .or_else(|| default_model.clone());
                if let Some(v) = v {
                    gen.insert("model_id".to_string(), v);
                }
            }
        },
    );

    merge_into_surface(
        &mut surfaces,
        "title_generation",
        |tg| {
            let gen = ensure_generation(tg);
            if !gen.contains_key("model_id") {
                if let Some(v) = conversation_title_model.clone() {
                    if v.as_str().map(|s| !s.is_empty()).unwrap_or(true) {
                        gen.insert("model_id".to_string(), v);
                    }
                }
            }
            if !tg.contains_key("prompt") {
                if let Some(v) = conversation_title_prompt.clone() {
                    tg.insert("prompt".to_string(), v);
                }
            }
        },
    );

    merge_into_surface(
        &mut surfaces,
        "speech_to_text",
        |stt| {
            if !stt.contains_key("model_id") {
                if let Some(v) = speech_to_text_model.clone() {
                    stt.insert("model_id".to_string(), v);
                }
            }
            if !stt.contains_key("language") {
                if let Some(v) = stt_language.clone() {
                    stt.insert("language".to_string(), v);
                }
            }
            if !stt.contains_key("keyterms_file") {
                if let Some(v) = stt_keyterms_file.clone() {
                    stt.insert("keyterms_file".to_string(), v);
                }
            }
            if !stt.contains_key("no_verbatim") {
                if let Some(v) = stt_no_verbatim.clone() {
                    stt.insert("no_verbatim".to_string(), v);
                }
            }
            if !stt.contains_key("prompt") {
                if let Some(v) = stt_prompt.clone() {
                    stt.insert("prompt".to_string(), v);
                }
            }
        },
    );

    obj.insert(
        "surfaces".to_string(),
        serde_json::Value::Object(surfaces),
    );

    let mut prompt_base = obj
        .remove("prompt_base")
        .and_then(|v| v.as_object().cloned())
        .unwrap_or_default();
    if !prompt_base.contains_key("system_prompt") {
        if let Some(v) = system_prompt {
            prompt_base.insert("system_prompt".to_string(), v);
        }
    }
    if !prompt_base.contains_key("about_me") {
        if let Some(v) = about_me {
            prompt_base.insert("about_me".to_string(), v);
        }
    }
    if !prompt_base.contains_key("environment_section") {
        if let Some(v) = environment_section {
            prompt_base.insert("environment_section".to_string(), v);
        }
    }
    obj.insert(
        "prompt_base".to_string(),
        serde_json::Value::Object(prompt_base),
    );

    if let Some(models) = obj.get_mut("models").and_then(|v| v.as_array_mut()) {
        for model in models {
            if let Some(m) = model.as_object_mut() {
                m.remove("enabled_tools");
                m.remove("language");
                m.remove("keyterms_file");
                m.remove("no_verbatim");
                let is_text = m.get("type").and_then(|t| t.as_str()).unwrap_or("text") == "text";
                if is_text {
                    if let Some(params) = m.get_mut("parameters").and_then(|p| p.as_object_mut()) {
                        params.remove("reasoning_effort");
                    }
                }
            }
        }
    }

    raw
}

fn extract_model_level_fields(
    obj: &serde_json::Map<String, serde_json::Value>,
    stt_model_id: Option<&serde_json::Value>,
) -> (
    Option<serde_json::Value>,
    Option<serde_json::Value>,
    Option<serde_json::Value>,
    Option<serde_json::Value>,
) {
    let stt_id = stt_model_id.and_then(|v| v.as_str());

    let Some(models) = obj.get("models").and_then(|m| m.as_array()) else {
        return (None, None, None, None);
    };

    let mut language = None;
    let mut keyterms_file = None;
    let mut no_verbatim = None;
    let mut chat_reasoning = None;

    for model in models {
        let Some(m) = model.as_object() else { continue };
        let model_id = m.get("id").and_then(|v| v.as_str()).unwrap_or("");
        let model_type = m.get("type").and_then(|v| v.as_str()).unwrap_or("text");

        if model_type == "stt" && Some(model_id) == stt_id {
            language = m.get("language").cloned();
            keyterms_file = m.get("keyterms_file").cloned();
            no_verbatim = m.get("no_verbatim").cloned();
        }

        if model_type == "text" && chat_reasoning.is_none() {
            if let Some(effort) = m
                .get("parameters")
                .and_then(|p| p.as_object())
                .and_then(|p| p.get("reasoning_effort"))
            {
                if !effort.is_null() {
                    chat_reasoning = Some(effort.clone());
                }
            }
        }
    }

    (language, keyterms_file, no_verbatim, chat_reasoning)
}

fn merge_into_surface<F>(
    surfaces: &mut serde_json::Map<String, serde_json::Value>,
    key: &str,
    mut apply: F,
) where
    F: FnMut(&mut serde_json::Map<String, serde_json::Value>),
{
    let entry = surfaces
        .entry(key.to_string())
        .or_insert_with(|| serde_json::Value::Object(serde_json::Map::new()));
    if let Some(obj) = entry.as_object_mut() {
        apply(obj);
    }
}

fn ensure_generation(
    surface: &mut serde_json::Map<String, serde_json::Value>,
) -> &mut serde_json::Map<String, serde_json::Value> {
    let entry = surface
        .entry("generation".to_string())
        .or_insert_with(|| serde_json::Value::Object(serde_json::Map::new()));
    entry.as_object_mut().expect("generation must be object")
}

fn ensure_parameters(
    gen: &mut serde_json::Map<String, serde_json::Value>,
) -> &mut serde_json::Map<String, serde_json::Value> {
    let entry = gen
        .entry("parameters".to_string())
        .or_insert_with(|| serde_json::Value::Object(serde_json::Map::new()));
    entry.as_object_mut().expect("parameters must be object")
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

    if let Some(ref stt_id) = settings.surfaces.speech_to_text.model_id {
        match settings.models.iter().find(|m| &m.id == stt_id) {
            None => {
                return Err(ConfigError::InvalidSettings(format!(
                    "speech_to_text.model_id references unknown model id: '{}'",
                    stt_id
                )));
            }
            Some(m) if !m.is_stt() => {
                return Err(ConfigError::InvalidSettings(format!(
                    "speech_to_text.model_id '{}' must have type='stt'",
                    stt_id
                )));
            }
            _ => {}
        }
    }

    validate_webview_providers(settings)?;

    Ok(())
}

fn validate_webview_providers(settings: &Settings) -> Result<(), ConfigError> {
    let mut seen = std::collections::HashSet::new();
    for provider in &settings.webview_providers {
        if provider.id.is_empty() {
            return Err(ConfigError::InvalidSettings(
                "webview_providers entry missing 'id'".to_string(),
            ));
        }
        if !provider
            .id
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '-')
        {
            return Err(ConfigError::InvalidSettings(format!(
                "webview_providers id '{}' must match [a-zA-Z0-9_-]+",
                provider.id
            )));
        }
        if !seen.insert(provider.id.clone()) {
            return Err(ConfigError::InvalidSettings(format!(
                "Duplicate webview_providers id: '{}'",
                provider.id
            )));
        }
        if provider.name.is_empty() {
            return Err(ConfigError::InvalidSettings(format!(
                "webview_providers '{}' has empty 'name'",
                provider.id
            )));
        }
        match tauri::Url::parse(&provider.url) {
            Ok(parsed) => {
                let scheme = parsed.scheme();
                if scheme != "http" && scheme != "https" {
                    return Err(ConfigError::InvalidSettings(format!(
                        "webview_providers '{}' url must use http or https scheme",
                        provider.id
                    )));
                }
            }
            Err(e) => {
                return Err(ConfigError::InvalidSettings(format!(
                    "webview_providers '{}' has invalid url: {}",
                    provider.id, e
                )));
            }
        }
    }
    Ok(())
}

pub fn migrate_model_params(settings: &mut Settings) {
    for model in &mut settings.models {
        if model.is_text() && model.parameters.is_none() {
            model.parameters = Some(Default::default());
        }
    }
}

fn ensure_surface_defaults(settings: &mut Settings) {
    let fallback = settings
        .models
        .iter()
        .find(|m| m.is_text())
        .map(|m| m.id.clone());

    let chat_model = settings.surfaces.chat.generation.model_id.clone();
    let chat_model = chat_model.filter(|id| settings.models.iter().any(|m| &m.id == id));
    if chat_model.is_none() {
        settings.surfaces.chat.generation.model_id = fallback.clone();
    } else {
        settings.surfaces.chat.generation.model_id = chat_model;
    }

    if settings.surfaces.quick_actions.generation.model_id.is_none() {
        settings.surfaces.quick_actions.generation.model_id =
            settings.surfaces.chat.generation.model_id.clone();
    }

    if settings.surfaces.title_generation.generation.model_id.is_none() {
        settings.surfaces.title_generation.generation.model_id =
            settings.surfaces.chat.generation.model_id.clone();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::settings::{ModelType, Provider};
    use std::fs;
    use tempfile::TempDir;

    fn test_model(id: &str, api_key: &str) -> ModelConfig {
        ModelConfig {
            id: id.to_string(),
            model: "test".to_string(),
            display_name: "Test".to_string(),
            model_type: ModelType::Text,
            provider: Some(Provider::default()),
            group: None,
            api_key: Some(api_key.to_string()),
            base_url: None,
            parameters: None,
            context_window_size: None,
            api_mode: None,
            store: true,
        }
    }

    fn test_stt_model(id: &str, api_key: &str) -> ModelConfig {
        ModelConfig {
            id: id.to_string(),
            model: "whisper-1".to_string(),
            display_name: "STT".to_string(),
            model_type: ModelType::Stt,
            provider: None,
            group: None,
            api_key: Some(api_key.to_string()),
            base_url: None,
            parameters: None,
            context_window_size: None,
            api_mode: None,
            store: true,
        }
    }

    fn setup_test_dir() -> TempDir {
        let dir = TempDir::new().unwrap();
        let default_json = include_str!("../../resources/default_settings.json");
        fs::write(dir.path().join("settings.json"), default_json).unwrap();
        dir
    }

    #[test]
    fn test_load_default_settings() {
        let dir = setup_test_dir();
        let service = ConfigService::load(dir.path(), None).expect("should load default settings");
        assert!(!service.settings().models.is_empty());
        assert!(service.settings().models.iter().any(|m| m.is_text()));
        assert!(service.settings().models.iter().any(|m| m.is_stt()));
        assert!(service.settings().surfaces.chat.generation.model_id.is_some());
        assert!(service.settings().surfaces.speech_to_text.model_id.is_some());
    }

    #[test]
    fn test_resolve_stt_model_returns_stt_entry() {
        let dir = setup_test_dir();
        let service = ConfigService::load(dir.path(), None).expect("load");
        let stt = service.resolve_stt_model().expect("stt model should resolve");
        assert!(stt.is_stt());
        assert_eq!(service.settings().surfaces.speech_to_text.language.as_deref(), Some("pl"));
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
        let text_model = models.iter().find(|m| m["type"] == "text").unwrap();
        assert_eq!(text_model["api_key"].as_str().unwrap(), "${OPENAI_API_KEY}");
        let stt_model = models.iter().find(|m| m["type"] == "stt").unwrap();
        assert_eq!(stt_model["api_key"].as_str().unwrap(), "${OPENAI_API_KEY}");
    }

    #[test]
    fn test_validate_speech_to_text_model_unknown_id() {
        let mut settings = Settings {
            models: vec![test_model("1", "${KEY}")],
            ..Default::default()
        };
        settings.surfaces.speech_to_text.model_id = Some("missing-stt".to_string());
        let result = validate(&settings);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("unknown model id"));
    }

    #[test]
    fn test_validate_speech_to_text_model_wrong_type() {
        let mut settings = Settings {
            models: vec![test_model("text-1", "${KEY}")],
            ..Default::default()
        };
        settings.surfaces.speech_to_text.model_id = Some("text-1".to_string());
        let result = validate(&settings);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("type='stt'"));
    }

    #[test]
    fn test_delete_model_clears_references() {
        let dir = setup_test_dir();
        let mut service = ConfigService::load(dir.path(), None).expect("load");
        service.add_model(test_stt_model("stt-to-delete", "${KEY}"));
        service.settings_mut().surfaces.speech_to_text.model_id = Some("stt-to-delete".to_string());

        service.delete_model("stt-to-delete");
        assert_eq!(service.settings().surfaces.speech_to_text.model_id, None);
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
        let default_json = include_str!("../../resources/default_settings.json");
        fs::write(resource_settings_dir.join("default_settings.json"), default_json).unwrap();

        let service = ConfigService::load(dir.path(), Some(resource_dir.path())).expect("should load");
        assert!(dir.path().join("settings.json").exists());
        assert!(!service.settings().models.is_empty());
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

    #[test]
    fn test_migrates_legacy_schema() {
        let dir = TempDir::new().unwrap();
        let legacy = r#"{
            "default_model": "model-a",
            "quick_action_default_model": "model-b",
            "speech_to_text_model": "stt-a",
            "conversation_title_model": "model-c",
            "conversation_title_prompt": "Make a title",
            "system_prompt": "Custom system",
            "about_me": "about_me.md",
            "environment_section": "environment_section.md",
            "stt_prompt": "stt_prompt.md",
            "selected_tools": ["web_search"],
            "models": [
                {
                    "id": "model-a",
                    "type": "text",
                    "model": "gpt-4",
                    "display_name": "GPT-4",
                    "provider": "openai",
                    "api_key": "${OPENAI_API_KEY}",
                    "parameters": { "reasoning_effort": "medium" },
                    "enabled_tools": ["web_search"]
                },
                {
                    "id": "model-b",
                    "type": "text",
                    "model": "gpt-4",
                    "display_name": "Quick",
                    "provider": "openai",
                    "api_key": "${OPENAI_API_KEY}"
                },
                {
                    "id": "model-c",
                    "type": "text",
                    "model": "gpt-4",
                    "display_name": "Title",
                    "provider": "openai",
                    "api_key": "${OPENAI_API_KEY}"
                },
                {
                    "id": "stt-a",
                    "type": "stt",
                    "model": "whisper-1",
                    "display_name": "STT",
                    "api_key": "${OPENAI_API_KEY}",
                    "language": "pl",
                    "keyterms_file": "keyterms.txt",
                    "no_verbatim": true
                }
            ]
        }"#;

        fs::write(dir.path().join("settings.json"), legacy).unwrap();
        let service = ConfigService::load(dir.path(), None).expect("load migrated");
        let s = service.settings();

        assert_eq!(s.surfaces.chat.generation.model_id.as_deref(), Some("model-a"));
        assert_eq!(s.surfaces.chat.generation.parameters.reasoning_effort.as_deref(), Some("medium"));
        assert_eq!(s.surfaces.chat.generation.enabled_tools, vec!["web_search"]);
        assert_eq!(s.prompt_base.system_prompt, "Custom system");
        assert_eq!(s.prompt_base.about_me.as_deref(), Some("about_me.md"));
        assert_eq!(s.prompt_base.environment_section.as_deref(), Some("environment_section.md"));
        assert_eq!(s.surfaces.quick_actions.generation.model_id.as_deref(), Some("model-b"));
        assert_eq!(s.surfaces.title_generation.generation.model_id.as_deref(), Some("model-c"));
        assert_eq!(s.surfaces.title_generation.prompt, "Make a title");
        assert_eq!(s.surfaces.speech_to_text.model_id.as_deref(), Some("stt-a"));
        assert_eq!(s.surfaces.speech_to_text.language.as_deref(), Some("pl"));
        assert_eq!(s.surfaces.speech_to_text.keyterms_file.as_deref(), Some("keyterms.txt"));
        assert_eq!(s.surfaces.speech_to_text.no_verbatim, Some(true));
        assert_eq!(s.surfaces.speech_to_text.prompt.as_deref(), Some("stt_prompt.md"));

        let text_a = s.models.iter().find(|m| m.id == "model-a").unwrap();
        assert_eq!(text_a.parameters.as_ref().unwrap().reasoning_effort, None);

        let saved = fs::read_to_string(dir.path().join("settings.json")).unwrap();
        let saved_json: serde_json::Value = serde_json::from_str(&saved).unwrap();
        assert!(saved_json.get("default_model").is_none());
        assert!(saved_json.get("system_prompt").is_none());
        assert!(saved_json.get("stt_prompt").is_none());
        assert!(saved_json.get("surfaces").is_some());
    }

    #[test]
    fn test_migrate_minimal_legacy() {
        let dir = TempDir::new().unwrap();
        let legacy = r#"{
            "default_model": "m1",
            "models": [
                { "id": "m1", "type": "text", "model": "gpt-4", "display_name": "M1", "provider": "openai", "api_key": "${OPENAI_API_KEY}" },
                { "id": "stt-only", "type": "stt", "model": "whisper-1", "display_name": "STT", "api_key": "${OPENAI_API_KEY}" }
            ]
        }"#;
        fs::write(dir.path().join("settings.json"), legacy).unwrap();
        let service = ConfigService::load(dir.path(), None).expect("load");
        let s = service.settings();
        assert_eq!(s.surfaces.chat.generation.model_id.as_deref(), Some("m1"));
        assert_eq!(s.surfaces.quick_actions.generation.model_id.as_deref(), Some("m1"));
    }

    #[test]
    fn test_migrate_chat_prompt_fields_into_prompt_base() {
        let dir = TempDir::new().unwrap();
        let legacy = r#"{
            "surfaces": {
                "chat": {
                    "generation": { "model_id": "m1", "parameters": {}, "enabled_tools": [] },
                    "system_prompt": "Custom system",
                    "about_me": "about_me.md",
                    "environment_section": "environment_section.md"
                },
                "speech_to_text": { "model_id": "stt-1" }
            },
            "models": [
                { "id": "m1", "type": "text", "model": "gpt-4", "display_name": "T", "provider": "openai", "api_key": "${OPENAI_API_KEY}" },
                { "id": "stt-1", "type": "stt", "model": "whisper-1", "display_name": "STT", "api_key": "${OPENAI_API_KEY}" }
            ]
        }"#;
        fs::write(dir.path().join("settings.json"), legacy).unwrap();
        let service = ConfigService::load(dir.path(), None).expect("load");
        let s = service.settings();

        assert_eq!(s.prompt_base.system_prompt, "Custom system");
        assert_eq!(s.prompt_base.about_me.as_deref(), Some("about_me.md"));
        assert_eq!(
            s.prompt_base.environment_section.as_deref(),
            Some("environment_section.md")
        );
        assert_eq!(s.surfaces.chat.generation.model_id.as_deref(), Some("m1"));

        let saved = fs::read_to_string(dir.path().join("settings.json")).unwrap();
        let saved_json: serde_json::Value = serde_json::from_str(&saved).unwrap();
        let chat = saved_json["surfaces"]["chat"].as_object().unwrap();
        assert!(!chat.contains_key("system_prompt"));
        assert!(!chat.contains_key("about_me"));
        assert!(!chat.contains_key("environment_section"));
        let prompt_base = saved_json["prompt_base"].as_object().unwrap();
        assert_eq!(prompt_base["system_prompt"], "Custom system");
    }

    #[test]
    fn test_validate_webview_providers_duplicate_id() {
        use crate::models::settings::WebviewProvider;
        let mut settings = Settings {
            models: vec![test_model("1", "${KEY}")],
            ..Default::default()
        };
        settings.webview_providers = vec![
            WebviewProvider {
                id: "claude".into(),
                name: "Claude".into(),
                url: "https://claude.ai/".into(),
            },
            WebviewProvider {
                id: "claude".into(),
                name: "Other".into(),
                url: "https://other.example/".into(),
            },
        ];
        let err = validate(&settings).unwrap_err().to_string();
        assert!(err.contains("Duplicate webview_providers"), "got: {err}");
    }

    #[test]
    fn test_validate_webview_providers_invalid_id_chars() {
        use crate::models::settings::WebviewProvider;
        let mut settings = Settings {
            models: vec![test_model("1", "${KEY}")],
            ..Default::default()
        };
        settings.webview_providers = vec![WebviewProvider {
            id: "claude pro".into(),
            name: "Claude Pro".into(),
            url: "https://claude.ai/".into(),
        }];
        let err = validate(&settings).unwrap_err().to_string();
        assert!(err.contains("[a-zA-Z0-9_-]+"), "got: {err}");
    }

    #[test]
    fn test_validate_webview_providers_invalid_scheme() {
        use crate::models::settings::WebviewProvider;
        let mut settings = Settings {
            models: vec![test_model("1", "${KEY}")],
            ..Default::default()
        };
        settings.webview_providers = vec![WebviewProvider {
            id: "ftp-thing".into(),
            name: "FTP".into(),
            url: "ftp://example.com/".into(),
        }];
        let err = validate(&settings).unwrap_err().to_string();
        assert!(err.contains("http or https"), "got: {err}");
    }

    #[test]
    fn test_validate_webview_providers_empty_list_ok() {
        let mut settings = Settings {
            models: vec![test_model("1", "${KEY}")],
            ..Default::default()
        };
        settings.webview_providers.clear();
        validate(&settings).expect("empty list should validate");
    }

    #[test]
    fn test_migrate_stt_only_legacy() {
        let dir = TempDir::new().unwrap();
        let legacy = r#"{
            "speech_to_text_model": "stt-1",
            "models": [
                { "id": "text-1", "type": "text", "model": "gpt-4", "display_name": "T", "provider": "openai", "api_key": "${OPENAI_API_KEY}" },
                { "id": "stt-1", "type": "stt", "model": "whisper-1", "display_name": "STT", "api_key": "${OPENAI_API_KEY}", "language": "en" }
            ]
        }"#;
        fs::write(dir.path().join("settings.json"), legacy).unwrap();
        let service = ConfigService::load(dir.path(), None).expect("load");
        let s = service.settings();
        assert_eq!(s.surfaces.speech_to_text.model_id.as_deref(), Some("stt-1"));
        assert_eq!(s.surfaces.speech_to_text.language.as_deref(), Some("en"));
    }
}
