use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Settings {
    #[serde(default = "default_true")]
    pub show_tray_icon: bool,

    #[serde(default)]
    pub debug_mode: bool,

    #[serde(default = "default_code_theme")]
    pub code_theme: String,

    #[serde(default = "default_menu_section_order")]
    pub menu_section_order: Vec<String>,

    #[serde(default)]
    pub description_generator: DescriptionGenerator,

    #[serde(default)]
    pub notifications: NotificationSettings,

    #[serde(default)]
    pub speech_to_text_model: Option<SpeechToTextModel>,

    #[serde(default)]
    pub default_model: Option<String>,

    #[serde(default = "default_debounce_ms")]
    pub number_input_debounce_ms: u32,

    #[serde(default)]
    pub models: Vec<ModelConfig>,

    #[serde(default)]
    pub keymaps: Vec<KeymapGroup>,

    #[serde(default)]
    pub prompts: Vec<PromptData>,

    #[serde(default = "default_system_prompt")]
    pub system_prompt: String,

    #[serde(default)]
    pub skills_order: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelConfig {
    pub id: String,
    pub model: String,
    pub display_name: String,

    #[serde(default)]
    pub api_key_source: ApiKeySource,

    #[serde(default)]
    pub provider: Provider,

    #[serde(default)]
    pub api_key_env: Option<String>,

    #[serde(default)]
    pub api_key: Option<String>,

    #[serde(default)]
    pub base_url: Option<String>,

    #[serde(default)]
    pub parameters: Option<ModelParameters>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum ApiKeySource {
    #[default]
    Env,
    Direct,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum Provider {
    #[default]
    Openai,
    Anthropic,
    Gemini,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ModelParameters {
    #[serde(default)]
    pub temperature: Option<f64>,

    #[serde(default)]
    pub max_tokens: Option<u32>,

    #[serde(default)]
    pub top_p: Option<f64>,

    #[serde(default)]
    pub frequency_penalty: Option<f64>,

    #[serde(default)]
    pub presence_penalty: Option<f64>,

    #[serde(default)]
    pub reasoning_effort: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SpeechToTextModel {
    pub model: String,
    pub display_name: String,
    pub api_key_env: String,

    #[serde(default)]
    pub base_url: Option<String>,

    #[serde(default)]
    pub api_key: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PromptData {
    pub id: String,
    pub name: String,

    #[serde(default)]
    pub description: Option<String>,

    #[serde(default)]
    pub messages: Vec<PromptMessage>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PromptMessage {
    pub role: String,
    pub content: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KeymapGroup {
    pub context: String,
    pub bindings: HashMap<String, String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NotificationSettings {
    #[serde(default)]
    pub events: NotificationEvents,

    #[serde(default)]
    pub background_colors: NotificationColors,

    #[serde(default = "default_true")]
    pub monochromatic_notification_icons: bool,

    #[serde(default)]
    pub opacity: Option<f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NotificationEvents {
    #[serde(default = "default_true")]
    pub prompt_execution_success: bool,
    #[serde(default = "default_true")]
    pub prompt_execution_cancel: bool,
    #[serde(default = "default_true")]
    pub prompt_execution_in_progress: bool,
    #[serde(default = "default_true")]
    pub speech_recording_start: bool,
    #[serde(default = "default_true")]
    pub speech_recording_stop: bool,
    #[serde(default = "default_true")]
    pub speech_transcription_success: bool,
    #[serde(default = "default_true")]
    pub context_saved: bool,
    #[serde(default = "default_true")]
    pub context_set: bool,
    #[serde(default = "default_true")]
    pub context_append: bool,
    #[serde(default = "default_true")]
    pub context_cleared: bool,
    #[serde(default = "default_true")]
    pub clipboard_copy: bool,
    #[serde(default = "default_true")]
    pub image_added: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NotificationColors {
    #[serde(default = "default_white")]
    pub success: String,
    #[serde(default = "default_white")]
    pub error: String,
    #[serde(default = "default_white")]
    pub info: String,
    #[serde(default = "default_white")]
    pub warning: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DescriptionGenerator {
    #[serde(default)]
    pub model: String,

    #[serde(default)]
    pub system_prompt: Option<String>,

    #[serde(default)]
    pub prompt: Option<String>,
}

fn default_true() -> bool {
    true
}

fn default_code_theme() -> String {
    "paraiso-dark".to_string()
}

fn default_menu_section_order() -> Vec<String> {
    vec![
        "ContextMenuProvider".to_string(),
        "LastInteractionMenuProvider".to_string(),
        "prompts".to_string(),
        "SpeechMenuProvider".to_string(),
        "settings".to_string(),
    ]
}

fn default_system_prompt() -> String {
    "You are a helpful assistant.".to_string()
}

fn default_debounce_ms() -> u32 {
    200
}

fn default_white() -> String {
    "#FFFFFF".to_string()
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            show_tray_icon: true,
            debug_mode: false,
            code_theme: default_code_theme(),
            menu_section_order: default_menu_section_order(),
            description_generator: DescriptionGenerator::default(),
            notifications: NotificationSettings::default(),
            speech_to_text_model: None,
            default_model: None,
            number_input_debounce_ms: 200,
            models: Vec::new(),
            keymaps: Vec::new(),
            prompts: Vec::new(),
            system_prompt: default_system_prompt(),
            skills_order: Vec::new(),
        }
    }
}

impl Default for NotificationSettings {
    fn default() -> Self {
        Self {
            events: NotificationEvents::default(),
            background_colors: NotificationColors::default(),
            monochromatic_notification_icons: true,
            opacity: None,
        }
    }
}

impl Default for NotificationEvents {
    fn default() -> Self {
        Self {
            prompt_execution_success: true,
            prompt_execution_cancel: true,
            prompt_execution_in_progress: true,
            speech_recording_start: true,
            speech_recording_stop: true,
            speech_transcription_success: true,
            context_saved: true,
            context_set: true,
            context_append: true,
            context_cleared: true,
            clipboard_copy: true,
            image_added: true,
        }
    }
}

impl Default for NotificationColors {
    fn default() -> Self {
        Self {
            success: "#FFFFFF".to_string(),
            error: "#FFFFFF".to_string(),
            info: "#FFFFFF".to_string(),
            warning: "#FFFFFF".to_string(),
        }
    }
}

impl Default for DescriptionGenerator {
    fn default() -> Self {
        Self {
            model: String::new(),
            system_prompt: None,
            prompt: None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_settings() {
        let settings = Settings::default();
        assert!(settings.show_tray_icon);
        assert!(!settings.debug_mode);
        assert_eq!(settings.code_theme, "paraiso-dark");
        assert_eq!(settings.number_input_debounce_ms, 200);
        assert_eq!(settings.menu_section_order.len(), 5);
        assert!(settings.models.is_empty());
        assert!(settings.prompts.is_empty());
    }

    #[test]
    fn test_deserialize_example_settings() {
        let json = include_str!("../../../../promptheus/settings_example/settings.json");
        let settings: Settings = serde_json::from_str(json).expect("failed to deserialize example settings.json");

        assert!(settings.show_tray_icon);
        assert!(!settings.debug_mode);
        assert_eq!(settings.code_theme, "paraiso-dark");
        assert_eq!(settings.models.len(), 1);
        assert_eq!(settings.models[0].model, "gpt-5.4");
        assert_eq!(settings.models[0].api_key_source, ApiKeySource::Env);
        assert_eq!(settings.keymaps.len(), 3);
        assert_eq!(settings.prompts.len(), 5);
        assert_eq!(settings.number_input_debounce_ms, 200);
        assert_eq!(settings.default_model, Some("13b85c38-19cc-4387-a52d-6577478be057".to_string()));

        assert!(settings.notifications.events.prompt_execution_success);
        assert!(settings.notifications.monochromatic_notification_icons);
        assert_eq!(settings.notifications.background_colors.success, "#FFFFFF");

        let speech = settings.speech_to_text_model.expect("speech model should be present");
        assert_eq!(speech.model, "gpt-4o-transcribe");
        assert_eq!(speech.api_key_env, "OPENAI_API_KEY");

        assert_eq!(settings.description_generator.model, "");
        assert!(settings.description_generator.prompt.is_some());
    }

    #[test]
    fn test_round_trip() {
        let json = include_str!("../../../../promptheus/settings_example/settings.json");
        let settings: Settings = serde_json::from_str(json).expect("deserialize");
        let serialized = serde_json::to_string_pretty(&settings).expect("serialize");
        let settings2: Settings = serde_json::from_str(&serialized).expect("re-deserialize");

        assert_eq!(settings.show_tray_icon, settings2.show_tray_icon);
        assert_eq!(settings.debug_mode, settings2.debug_mode);
        assert_eq!(settings.code_theme, settings2.code_theme);
        assert_eq!(settings.models.len(), settings2.models.len());
        assert_eq!(settings.models[0].id, settings2.models[0].id);
        assert_eq!(settings.models[0].model, settings2.models[0].model);
        assert_eq!(settings.prompts.len(), settings2.prompts.len());
        assert_eq!(settings.keymaps.len(), settings2.keymaps.len());
        assert_eq!(settings.default_model, settings2.default_model);
        assert_eq!(settings.number_input_debounce_ms, settings2.number_input_debounce_ms);
    }

    #[test]
    fn test_deserialize_empty_json() {
        let settings: Settings = serde_json::from_str("{}").expect("empty JSON should use defaults");
        assert!(settings.show_tray_icon);
        assert!(!settings.debug_mode);
        assert_eq!(settings.code_theme, "paraiso-dark");
        assert_eq!(settings.number_input_debounce_ms, 200);
        assert!(settings.models.is_empty());
    }

    #[test]
    fn test_api_key_source_default() {
        let json = r#"{"id":"1","model":"test","display_name":"Test"}"#;
        let config: ModelConfig = serde_json::from_str(json).expect("deserialize model config");
        assert_eq!(config.api_key_source, ApiKeySource::Env);
    }
}
