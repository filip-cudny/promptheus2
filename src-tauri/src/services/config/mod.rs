mod defaults;
mod loader;
mod migrator;
mod path;
mod prompts;
#[cfg(test)]
mod tests;

pub use prompts::{PromptKind, PromptStore};

use std::path::{Path, PathBuf};

use crate::models::settings::{
    KeymapGroup, ModelConfig, NotificationSettings, Settings, SpeechToTextConfig,
};

use path::resolve_config_relative;

use defaults::{ensure_surface_defaults, validate};
use loader::initialize_defaults;
use migrator::{
    migrate_explicit_capabilities, migrate_model_params, parse_and_migrate,
    replace_legacy_system_default, sanitize_capabilities, InlinePromptWrite, LegacyFlatMdRename,
};

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

const MAX_PREFERRED_NAME_LEN: usize = 60;

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

        let content = std::fs::read_to_string(&settings_path)?;
        let outcome = parse_and_migrate(&content)?;
        let mut settings = outcome.settings;

        apply_inline_prompts(config_dir, &outcome.inline_prompts)?;
        apply_legacy_flat_md_renames(config_dir, &outcome.legacy_flat_md_renames);
        rewrite_legacy_default_paths(config_dir, &mut settings);
        ensure_prompt_defaults(config_dir, &settings);
        replace_legacy_system_default(config_dir, &settings.prompt_base.system);

        migrate_model_params(&mut settings);
        sanitize_capabilities(&mut settings);
        migrate_explicit_capabilities(&mut settings);
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
        let outcome = parse_and_migrate(&content)?;
        let mut settings = outcome.settings;

        apply_inline_prompts(&self.config_dir, &outcome.inline_prompts)?;
        apply_legacy_flat_md_renames(&self.config_dir, &outcome.legacy_flat_md_renames);
        rewrite_legacy_default_paths(&self.config_dir, &mut settings);
        ensure_prompt_defaults(&self.config_dir, &settings);
        replace_legacy_system_default(&self.config_dir, &settings.prompt_base.system);

        migrate_model_params(&mut settings);
        sanitize_capabilities(&mut settings);
        migrate_explicit_capabilities(&mut settings);
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
            "autosave_debounce_ms" => {
                if let Some(v) = value.as_u64() {
                    self.settings.autosave_debounce_ms = v as u32;
                }
            }
            "preferred_name" => {
                if let Some(v) = value.as_str() {
                    let trimmed = v.trim();
                    let clamped: String = trimmed.chars().take(MAX_PREFERRED_NAME_LEN).collect();
                    self.settings.preferred_name = clamped;
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

    pub fn update_skills_order(&mut self, order: Vec<String>) {
        self.settings.skills_order = order;
    }

    pub fn prompt_path(&self, kind: PromptKind) -> Option<&str> {
        match kind {
            PromptKind::System => Some(&self.settings.prompt_base.system),
            PromptKind::AboutYou => Some(&self.settings.prompt_base.about_you),
            PromptKind::Environment => Some(&self.settings.prompt_base.environment),
            PromptKind::InputFormat => Some(&self.settings.prompt_base.input_format),
            PromptKind::TitleGeneration => Some(&self.settings.surfaces.title_generation.prompt),
            PromptKind::SpeechToText => self.settings.surfaces.speech_to_text.prompt.as_deref(),
        }
    }

    pub fn read_prompt(&self, kind: PromptKind) -> String {
        let Some(path) = self.prompt_path(kind) else {
            return String::new();
        };
        PromptStore::new(&self.config_dir)
            .read(path)
            .unwrap_or_default()
    }

    pub fn read_prompt_optional(&self, kind: PromptKind) -> Option<String> {
        let path = self.prompt_path(kind)?;
        let content = PromptStore::new(&self.config_dir).read(path).ok()?;
        let trimmed = content.trim();
        if trimmed.is_empty() {
            None
        } else {
            Some(trimmed.to_string())
        }
    }

    pub fn write_prompt(&self, kind: PromptKind, content: &str) -> Result<(), ConfigError> {
        let Some(path) = self.prompt_path(kind) else {
            return Err(ConfigError::InvalidSettings(format!(
                "no path configured for prompt kind {:?}",
                kind
            )));
        };
        let path = path.to_string();
        PromptStore::new(&self.config_dir).write(&path, content)
    }

    pub fn input_format_guide(&self) -> String {
        self.read_prompt(PromptKind::InputFormat)
    }

    pub fn about_you(&self) -> String {
        self.read_prompt(PromptKind::AboutYou)
    }

    pub fn preferred_name(&self) -> &str {
        &self.settings.preferred_name
    }

    pub fn environment_section_template(&self) -> String {
        self.read_prompt(PromptKind::Environment)
    }

    pub fn system_prompt(&self) -> String {
        self.read_prompt(PromptKind::System)
    }

    pub fn title_generation_prompt(&self) -> String {
        self.read_prompt(PromptKind::TitleGeneration)
    }

    pub fn stt_prompt(&self) -> Option<String> {
        self.read_prompt_optional(PromptKind::SpeechToText)
    }

    pub fn stt_keyterms(&self) -> Vec<String> {
        let Some(raw) = self.settings.surfaces.speech_to_text.keyterms_file.as_deref() else {
            return Vec::new();
        };
        let path = match resolve_config_relative(raw, &self.config_dir) {
            Ok(p) => p,
            Err(e) => {
                log::warn!("invalid speech_to_text.keyterms_file '{raw}': {e}");
                return Vec::new();
            }
        };
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

fn apply_inline_prompts(
    config_dir: &Path,
    inline_prompts: &[InlinePromptWrite],
) -> Result<(), ConfigError> {
    if inline_prompts.is_empty() {
        return Ok(());
    }
    let store = PromptStore::new(config_dir);
    for entry in inline_prompts {
        let target = match store.resolve(&entry.path) {
            Ok(p) => p,
            Err(e) => {
                log::warn!(
                    "skipping inline prompt write to '{}': {e}",
                    entry.path
                );
                continue;
            }
        };
        if target.exists() {
            log::info!(
                "preserving existing prompt file at '{}', migrated inline content discarded",
                entry.path
            );
            continue;
        }
        store.write(&entry.path, &entry.content)?;
        log::info!("migrated inline prompt to '{}'", entry.path);
    }
    Ok(())
}

fn rewrite_legacy_default_paths(config_dir: &Path, settings: &mut Settings) {
    let rewrites: [(&mut String, &[&str], &str); 3] = [
        (
            &mut settings.prompt_base.about_you,
            &["about_me.md", "prompts/base/about_me.md"],
            "prompts/base/about_you.md",
        ),
        (
            &mut settings.prompt_base.environment,
            &["environment_section.md"],
            "prompts/base/environment.md",
        ),
        (
            &mut settings.prompt_base.input_format,
            &["input_format_guide.md"],
            "prompts/base/input_format.md",
        ),
    ];
    for (field, legacy_paths, new_path) in rewrites {
        if legacy_paths.iter().any(|p| field == *p) && config_dir.join(new_path).exists() {
            log::info!(
                "rewriting legacy default prompt path '{}' -> '{}'",
                field,
                new_path
            );
            *field = new_path.to_string();
        }
    }
}

fn apply_legacy_flat_md_renames(config_dir: &Path, renames: &[LegacyFlatMdRename]) {
    for rename in renames {
        let old = config_dir.join(rename.old_relative);
        let new = config_dir.join(rename.new_relative);
        if !old.exists() || new.exists() {
            continue;
        }
        if let Some(parent) = new.parent() {
            if let Err(e) = std::fs::create_dir_all(parent) {
                log::warn!(
                    "failed to create parent dir for legacy rename '{}': {e}",
                    rename.new_relative
                );
                continue;
            }
        }
        match std::fs::rename(&old, &new) {
            Ok(_) => log::info!(
                "migrated legacy prompt file '{}' -> '{}'",
                rename.old_relative,
                rename.new_relative
            ),
            Err(e) => log::warn!(
                "failed to rename legacy prompt file '{}' -> '{}': {e}",
                rename.old_relative,
                rename.new_relative
            ),
        }
    }
}

fn ensure_prompt_defaults(config_dir: &Path, settings: &Settings) {
    let store = PromptStore::new(config_dir);
    let entries: [(PromptKind, Option<&str>); 6] = [
        (PromptKind::System, Some(settings.prompt_base.system.as_str())),
        (PromptKind::AboutYou, Some(settings.prompt_base.about_you.as_str())),
        (
            PromptKind::Environment,
            Some(settings.prompt_base.environment.as_str()),
        ),
        (
            PromptKind::InputFormat,
            Some(settings.prompt_base.input_format.as_str()),
        ),
        (
            PromptKind::TitleGeneration,
            Some(settings.surfaces.title_generation.prompt.as_str()),
        ),
        (
            PromptKind::SpeechToText,
            settings.surfaces.speech_to_text.prompt.as_deref(),
        ),
    ];
    for (kind, raw_path) in entries {
        let path = raw_path.unwrap_or_else(|| kind.default_path());
        if let Err(e) = store.ensure_default(kind, path) {
            log::warn!(
                "failed to ensure default prompt for {:?} at '{}': {e}",
                kind,
                path
            );
        }
    }
}
