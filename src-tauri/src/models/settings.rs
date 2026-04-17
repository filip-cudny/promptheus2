use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use crate::services::env_resolve::resolve_env_refs;

const fn default_timeout_secs() -> u64 {
    180
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpServerConfig {
    pub command: String,
    #[serde(default)]
    pub args: Vec<String>,
    #[serde(default)]
    pub env: HashMap<String, String>,
    #[serde(default = "default_timeout_secs")]
    pub timeout_secs: u64,
    #[serde(default)]
    pub tool_timeouts: HashMap<String, u64>,
}

impl McpServerConfig {
    pub fn resolved_command(&self) -> String {
        resolve_env_refs(&self.command)
    }

    pub fn resolved_args(&self) -> Vec<String> {
        self.args.iter().map(|a| resolve_env_refs(a)).collect()
    }

    pub fn resolved_env(&self) -> HashMap<String, String> {
        self.env
            .iter()
            .map(|(k, v)| (k.clone(), resolve_env_refs(v)))
            .collect()
    }
}

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
    pub speech_to_text_model: Option<String>,

    #[serde(default)]
    pub default_model: Option<String>,

    #[serde(default)]
    pub quick_action_default_model: Option<String>,

    #[serde(default = "default_debounce_ms")]
    pub number_input_debounce_ms: u32,

    #[serde(default)]
    pub models: Vec<ModelConfig>,

    #[serde(default)]
    pub keymaps: Vec<KeymapGroup>,

    #[serde(default = "default_system_prompt")]
    pub system_prompt: String,

    #[serde(default)]
    pub about_me: Option<String>,

    #[serde(default)]
    pub environment_section: Option<String>,

    #[serde(default)]
    pub stt_prompt: Option<String>,

    #[serde(default = "default_recent_apps_count")]
    pub recent_apps_count: usize,

    #[serde(default)]
    pub mcp_servers: HashMap<String, McpServerConfig>,

    #[serde(default)]
    pub skills_order: Vec<String>,

    #[serde(default)]
    pub conversation_title_model: String,

    #[serde(default = "default_conversation_title_prompt")]
    pub conversation_title_prompt: String,

    #[serde(default)]
    pub selected_tools: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum ModelType {
    #[default]
    Text,
    Stt,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelConfig {
    pub id: String,
    pub model: String,
    pub display_name: String,

    #[serde(default, rename = "type")]
    pub model_type: ModelType,

    #[serde(default)]
    pub provider: Option<Provider>,

    #[serde(default)]
    pub group: Option<String>,

    #[serde(default)]
    pub api_key: Option<String>,

    #[serde(default)]
    pub base_url: Option<String>,

    #[serde(default)]
    pub parameters: Option<ModelParameters>,

    #[serde(default)]
    pub context_window_size: Option<u32>,

    #[serde(default)]
    pub api_mode: Option<ApiMode>,

    #[serde(default = "default_true")]
    pub store: bool,

    #[serde(default)]
    pub enabled_tools: Vec<String>,

    #[serde(default)]
    pub language: Option<String>,

    #[serde(default)]
    pub keyterms_file: Option<String>,

    #[serde(default)]
    pub no_verbatim: Option<bool>,

    #[serde(default, skip_serializing)]
    pub api_key_source: Option<String>,

    #[serde(default, skip_serializing)]
    pub api_key_env: Option<String>,
}

impl ModelConfig {
    pub fn resolved_api_key(&self) -> Option<String> {
        self.api_key.as_ref().map(|k| resolve_env_refs(k)).filter(|k| !k.is_empty())
    }

    pub fn is_text(&self) -> bool {
        matches!(self.model_type, ModelType::Text)
    }

    pub fn is_stt(&self) -> bool {
        matches!(self.model_type, ModelType::Stt)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum Provider {
    #[default]
    Openai,
    Anthropic,
    Gemini,
    ElevenLabs,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum ApiMode {
    #[default]
    Responses,
    Completions,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
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

    #[serde(flatten)]
    pub extra: HashMap<String, serde_json::Value>,
}

impl Default for ModelParameters {
    fn default() -> Self {
        Self {
            temperature: None,
            max_tokens: None,
            top_p: None,
            frequency_penalty: None,
            presence_penalty: None,
            reasoning_effort: None,
            extra: HashMap::new(),
        }
    }
}

impl ModelParameters {
    pub fn active_known_params(&self) -> Vec<&'static str> {
        let mut active = Vec::new();
        if self.temperature.is_some() { active.push("temperature"); }
        if self.max_tokens.is_some() { active.push("max_tokens"); }
        if self.top_p.is_some() { active.push("top_p"); }
        if self.frequency_penalty.is_some() { active.push("frequency_penalty"); }
        if self.presence_penalty.is_some() { active.push("presence_penalty"); }
        if self.reasoning_effort.is_some() { active.push("reasoning_effort"); }
        active
    }

    pub fn from_map(map: &HashMap<String, serde_json::Value>) -> Self {
        let mut params = Self::default();
        let mut extra = HashMap::new();
        for (key, value) in map {
            match key.as_str() {
                "temperature" => params.temperature = value.as_f64(),
                "max_tokens" => params.max_tokens = value.as_u64().map(|v| v as u32),
                "top_p" => params.top_p = value.as_f64(),
                "frequency_penalty" => params.frequency_penalty = value.as_f64(),
                "presence_penalty" => params.presence_penalty = value.as_f64(),
                "reasoning_effort" => params.reasoning_effort = value.as_str().map(String::from),
                _ => { extra.insert(key.clone(), value.clone()); }
            }
        }
        params.extra = extra;
        params
    }
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
        "chat".to_string(),
        "skills".to_string(),
        "speech".to_string(),
        "lastInteraction".to_string(),
        "context".to_string(),
        "settings".to_string(),
    ]
}

fn default_system_prompt() -> String {
    "You are a helpful assistant.".to_string()
}

fn default_debounce_ms() -> u32 {
    200
}

fn default_conversation_title_prompt() -> String {
    "Generate a short conversation title based on the user's first message. Return only the title, 2-6 words, no emoji, no quotes, no trailing punctuation. Match the user's language when possible.".to_string()
}

fn default_recent_apps_count() -> usize {
    4
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
            quick_action_default_model: None,
            number_input_debounce_ms: 200,
            models: Vec::new(),
            keymaps: Vec::new(),
            system_prompt: default_system_prompt(),
            about_me: None,
            environment_section: None,
            stt_prompt: None,
            recent_apps_count: default_recent_apps_count(),
            mcp_servers: HashMap::new(),
            skills_order: Vec::new(),
            conversation_title_model: String::new(),
            conversation_title_prompt: default_conversation_title_prompt(),
            selected_tools: Vec::new(),
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
        assert_eq!(settings.menu_section_order.len(), 6);
        assert!(settings.models.is_empty());
    }

    #[test]
    fn test_deserialize_default_settings() {
        let json = include_str!("../../resources/default_settings.json");
        let settings: Settings = serde_json::from_str(json).expect("failed to deserialize default_settings.json");

        assert!(settings.show_tray_icon);
        assert!(!settings.debug_mode);
        assert_eq!(settings.code_theme, "paraiso-dark");
        assert_eq!(settings.keymaps.len(), 3);
        assert_eq!(settings.number_input_debounce_ms, 200);

        let stt_id = settings.speech_to_text_model.as_deref().expect("speech_to_text_model should reference an id");
        let stt_model = settings.models.iter().find(|m| m.id == stt_id).expect("stt model should be present in models list");
        assert!(stt_model.is_stt());

        assert!(settings.notifications.events.prompt_execution_success);
        assert!(settings.notifications.monochromatic_notification_icons);
        assert_eq!(settings.notifications.background_colors.success, "#FFFFFF");

        assert!(settings.description_generator.prompt.is_some());
    }

    #[test]
    fn test_round_trip() {
        let json = include_str!("../../resources/default_settings.json");
        let settings: Settings = serde_json::from_str(json).expect("deserialize");
        let serialized = serde_json::to_string_pretty(&settings).expect("serialize");
        let settings2: Settings = serde_json::from_str(&serialized).expect("re-deserialize");

        assert_eq!(settings.show_tray_icon, settings2.show_tray_icon);
        assert_eq!(settings.debug_mode, settings2.debug_mode);
        assert_eq!(settings.code_theme, settings2.code_theme);
        assert_eq!(settings.models.len(), settings2.models.len());
        assert_eq!(settings.models[0].id, settings2.models[0].id);
        assert_eq!(settings.models[0].model, settings2.models[0].model);
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
    fn test_deserialize_with_mcp_servers() {
        let json = r#"{
            "mcp_servers": {
                "my-tools": {
                    "command": "npx",
                    "args": ["-y", "my-mcp-server"],
                    "env": { "API_KEY": "test-key" }
                }
            }
        }"#;
        let settings: Settings = serde_json::from_str(json).expect("should deserialize with mcp_servers");
        assert_eq!(settings.mcp_servers.len(), 1);
        let server = &settings.mcp_servers["my-tools"];
        assert_eq!(server.command, "npx");
        assert_eq!(server.args, vec!["-y", "my-mcp-server"]);
        assert_eq!(server.env.get("API_KEY").unwrap(), "test-key");
    }

    #[test]
    fn test_mcp_resolved_env_with_env_refs() {
        std::env::set_var("TEST_MCP_ENV_REF", "from-env");
        let config = McpServerConfig {
            command: "test".to_string(),
            args: vec![],
            env: HashMap::from([
                ("EXPLICIT".to_string(), "direct".to_string()),
                ("FROM_ENV".to_string(), "${TEST_MCP_ENV_REF}".to_string()),
            ]),
            timeout_secs: 180,
            tool_timeouts: HashMap::new(),
        };
        let resolved = config.resolved_env();
        assert_eq!(resolved.get("EXPLICIT").unwrap(), "direct");
        assert_eq!(resolved.get("FROM_ENV").unwrap(), "from-env");
        std::env::remove_var("TEST_MCP_ENV_REF");
    }

    #[test]
    fn test_mcp_resolved_command_and_args() {
        std::env::set_var("TEST_MCP_PATH", "/opt/tools");
        let config = McpServerConfig {
            command: "${TEST_MCP_PATH}/bin/server".to_string(),
            args: vec!["--config".to_string(), "${TEST_MCP_PATH}/config.json".to_string()],
            env: HashMap::new(),
            timeout_secs: 180,
            tool_timeouts: HashMap::new(),
        };
        assert_eq!(config.resolved_command(), "/opt/tools/bin/server");
        assert_eq!(config.resolved_args(), vec!["--config", "/opt/tools/config.json"]);
        std::env::remove_var("TEST_MCP_PATH");
    }

    #[test]
    fn test_deserialize_without_mcp_servers() {
        let json = r#"{ "debug_mode": true }"#;
        let settings: Settings = serde_json::from_str(json).expect("should deserialize without mcp_servers");
        assert!(settings.mcp_servers.is_empty());
    }

    #[test]
    fn test_model_resolved_api_key_direct() {
        let config: ModelConfig = serde_json::from_str(r#"{"id":"1","model":"test","display_name":"Test","api_key":"sk-direct"}"#).unwrap();
        assert_eq!(config.resolved_api_key().as_deref(), Some("sk-direct"));
    }

    #[test]
    fn test_model_resolved_api_key_env_ref() {
        std::env::set_var("TEST_MODEL_KEY", "sk-from-env");
        let config: ModelConfig = serde_json::from_str(r#"{"id":"1","model":"test","display_name":"Test","api_key":"${TEST_MODEL_KEY}"}"#).unwrap();
        assert_eq!(config.resolved_api_key().as_deref(), Some("sk-from-env"));
        std::env::remove_var("TEST_MODEL_KEY");
    }

    #[test]
    fn test_model_resolved_api_key_missing_env() {
        let config: ModelConfig = serde_json::from_str(r#"{"id":"1","model":"test","display_name":"Test","api_key":"${DEFINITELY_MISSING_KEY_XYZ}"}"#).unwrap();
        assert_eq!(config.resolved_api_key(), None);
    }

    #[test]
    fn test_legacy_api_key_env_deserialized() {
        let config: ModelConfig = serde_json::from_str(r#"{"id":"1","model":"test","display_name":"Test","api_key_env":"OPENAI_API_KEY","api_key_source":"env"}"#).unwrap();
        assert_eq!(config.api_key_env.as_deref(), Some("OPENAI_API_KEY"));
        assert_eq!(config.api_key_source.as_deref(), Some("env"));
    }
}
