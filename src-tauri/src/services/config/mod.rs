mod defaults;
mod loader;
mod migrator;
#[cfg(test)]
mod tests;

use std::path::{Path, PathBuf};

use crate::models::settings::{
    KeymapGroup, ModelConfig, NotificationSettings, Settings, SpeechToTextConfig,
};
use crate::services::env_resolve::resolve_env_refs;

use defaults::{ensure_surface_defaults, validate};
use loader::{
    initialize_about_me, initialize_defaults, initialize_environment_section,
    initialize_input_format_guide,
};
use migrator::{migrate_model_params, parse_and_migrate};

pub use loader::load_env;

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

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Deserialize, serde::Serialize)]
#[serde(rename_all = "snake_case")]
pub enum SurfaceKind {
    Chat,
    QuickActions,
    TitleGeneration,
    SpeechToText,
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
            initialize_defaults(config_dir, resource_dir)?;
        }

        let _ = initialize_input_format_guide(config_dir);
        let _ = initialize_about_me(config_dir);
        let _ = initialize_environment_section(config_dir);

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
        if trimmed.is_empty() {
            None
        } else {
            Some(trimmed.to_string())
        }
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
