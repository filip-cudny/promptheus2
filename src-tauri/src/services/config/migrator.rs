use serde_json::Value;

use crate::models::capabilities::{Effort, ModelCapabilities, ReasoningMode};
use crate::models::settings::{Provider, Settings};
use crate::services::ai::capabilities::OPENAI_EFFORT_MODELS;

use super::defaults::{ensure_generation, ensure_parameters};
use super::prompts::PromptKind;
use super::ConfigError;

#[derive(Debug, Default)]
pub(super) struct MigrationOutcome {
    pub settings: Settings,
    pub inline_prompts: Vec<InlinePromptWrite>,
    pub legacy_flat_md_renames: Vec<LegacyFlatMdRename>,
}

#[derive(Debug, Clone)]
pub(super) struct InlinePromptWrite {
    pub path: String,
    pub content: String,
}

#[derive(Debug, Clone)]
pub(super) struct LegacyFlatMdRename {
    pub old_relative: &'static str,
    pub new_relative: &'static str,
}

const LEGACY_FLAT_MD_RENAMES: &[LegacyFlatMdRename] = &[
    LegacyFlatMdRename {
        old_relative: "about_me.md",
        new_relative: "prompts/base/about_me.md",
    },
    LegacyFlatMdRename {
        old_relative: "environment_section.md",
        new_relative: "prompts/base/environment.md",
    },
    LegacyFlatMdRename {
        old_relative: "input_format_guide.md",
        new_relative: "prompts/base/input_format.md",
    },
];

pub(super) fn parse_and_migrate(content: &str) -> Result<MigrationOutcome, ConfigError> {
    let raw: Value = serde_json::from_str(content)?;
    let needs_migration = raw
        .as_object()
        .map(|obj| {
            !obj.contains_key("surfaces")
                || has_legacy_fields(obj)
                || has_chat_prompt_base_fields(obj)
                || prompts_need_migration(obj)
        })
        .unwrap_or(false);

    let mut inline_prompts = Vec::new();

    let migrated = if needs_migration {
        log::info!("migrating settings.json to new schema");
        let mut migrated = migrate_legacy_json(raw);
        migrate_prompt_paths(&mut migrated, &mut inline_prompts);
        migrated
    } else {
        raw
    };

    let settings: Settings = serde_json::from_value(migrated)?;

    Ok(MigrationOutcome {
        settings,
        inline_prompts,
        legacy_flat_md_renames: LEGACY_FLAT_MD_RENAMES.to_vec(),
    })
}

pub(super) fn migrate_model_params(settings: &mut Settings) {
    for model in &mut settings.models {
        if model.is_text() && model.parameters.is_none() {
            model.parameters = Some(Default::default());
        }
    }
}

pub(super) fn migrate_explicit_capabilities(settings: &mut Settings) {
    for model in &mut settings.models {
        if model.capabilities.is_some() {
            continue;
        }
        if model.provider.as_ref() != Some(&Provider::Openai) {
            continue;
        }
        if !OPENAI_EFFORT_MODELS.contains(&model.model.as_str()) {
            continue;
        }
        model.capabilities = Some(ModelCapabilities {
            reasoning: ReasoningMode::Effort {
                allowed: vec![
                    Effort::None,
                    Effort::Minimal,
                    Effort::Low,
                    Effort::Medium,
                    Effort::High,
                    Effort::XHigh,
                ],
            },
        });
    }
}

pub(super) fn sanitize_capabilities(settings: &mut Settings) {
    for model in &mut settings.models {
        let Some(ref caps) = model.capabilities else {
            continue;
        };
        if let Err(reason) = caps.validate() {
            log::warn!(
                "model '{}' (id={}): ignoring invalid capabilities — {reason}",
                model.display_name,
                model.id
            );
            model.capabilities = None;
        }
    }
}

fn has_legacy_fields(obj: &serde_json::Map<String, Value>) -> bool {
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
        "description_generator",
    ];
    LEGACY_KEYS.iter().any(|k| obj.contains_key(*k))
}

fn has_chat_prompt_base_fields(obj: &serde_json::Map<String, Value>) -> bool {
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

fn prompts_need_migration(obj: &serde_json::Map<String, Value>) -> bool {
    if let Some(pb) = obj.get("prompt_base").and_then(|v| v.as_object()) {
        if pb.contains_key("system_prompt") || pb.contains_key("environment_section") {
            return true;
        }
        if !pb.contains_key("input_format") {
            return true;
        }
        for key in ["system", "about_me", "environment", "input_format"] {
            if let Some(v) = pb.get(key) {
                if value_is_inline_prompt(v) {
                    return true;
                }
            }
        }
    } else {
        return true;
    }

    if let Some(surfaces) = obj.get("surfaces").and_then(|v| v.as_object()) {
        if let Some(tg) = surfaces.get("title_generation").and_then(|v| v.as_object()) {
            if let Some(p) = tg.get("prompt") {
                if value_is_inline_prompt(p) {
                    return true;
                }
            }
        }
        if let Some(stt) = surfaces.get("speech_to_text").and_then(|v| v.as_object()) {
            if let Some(p) = stt.get("prompt") {
                if value_is_inline_prompt(p) {
                    return true;
                }
            }
        }
    }

    false
}

fn value_is_inline_prompt(v: &Value) -> bool {
    match v.as_str() {
        Some(s) => !looks_like_path(s),
        None => false,
    }
}

fn looks_like_path(s: &str) -> bool {
    let trimmed = s.trim();
    if trimmed.is_empty() {
        return false;
    }
    if trimmed.contains('\n') {
        return false;
    }
    if trimmed.len() > 256 {
        return false;
    }
    let lower = trimmed.to_ascii_lowercase();
    if lower.ends_with(".md") || lower.ends_with(".markdown") {
        return true;
    }
    if trimmed.starts_with('$') {
        return true;
    }
    false
}

fn migrate_legacy_json(mut raw: Value) -> Value {
    let Some(obj) = raw.as_object_mut() else {
        return raw;
    };

    obj.remove("description_generator");

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

    merge_into_surface(&mut surfaces, "chat", |chat| {
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
    });

    merge_into_surface(&mut surfaces, "quick_actions", |qa| {
        let gen = ensure_generation(qa);
        if !gen.contains_key("model_id") {
            let v = quick_action_default_model
                .clone()
                .or_else(|| default_model.clone());
            if let Some(v) = v {
                gen.insert("model_id".to_string(), v);
            }
        }
    });

    merge_into_surface(&mut surfaces, "title_generation", |tg| {
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
    });

    merge_into_surface(&mut surfaces, "speech_to_text", |stt| {
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
    });

    obj.insert("surfaces".to_string(), Value::Object(surfaces));

    let mut prompt_base = obj
        .remove("prompt_base")
        .and_then(|v| v.as_object().cloned())
        .unwrap_or_default();

    if let Some(v) = prompt_base.remove("system_prompt") {
        if !prompt_base.contains_key("system") {
            prompt_base.insert("system".to_string(), v);
        }
    }
    if let Some(v) = prompt_base.remove("environment_section") {
        if !prompt_base.contains_key("environment") {
            prompt_base.insert("environment".to_string(), v);
        }
    }

    if !prompt_base.contains_key("system") {
        if let Some(v) = system_prompt {
            prompt_base.insert("system".to_string(), v);
        }
    }
    if !prompt_base.contains_key("about_me") {
        if let Some(v) = about_me {
            prompt_base.insert("about_me".to_string(), v);
        }
    }
    if !prompt_base.contains_key("environment") {
        if let Some(v) = environment_section {
            prompt_base.insert("environment".to_string(), v);
        }
    }
    obj.insert("prompt_base".to_string(), Value::Object(prompt_base));

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

fn migrate_prompt_paths(raw: &mut Value, inline_prompts: &mut Vec<InlinePromptWrite>) {
    let Some(obj) = raw.as_object_mut() else {
        return;
    };

    let prompt_base = obj
        .entry("prompt_base".to_string())
        .or_insert_with(|| Value::Object(Default::default()));
    if let Some(pb) = prompt_base.as_object_mut() {
        normalize_prompt_field(pb, "system", PromptKind::System, inline_prompts);
        normalize_prompt_field(pb, "about_me", PromptKind::AboutMe, inline_prompts);
        normalize_prompt_field(pb, "environment", PromptKind::Environment, inline_prompts);
        normalize_prompt_field(
            pb,
            "input_format",
            PromptKind::InputFormat,
            inline_prompts,
        );
    }

    if let Some(surfaces) = obj.get_mut("surfaces").and_then(|v| v.as_object_mut()) {
        if let Some(tg) = surfaces
            .get_mut("title_generation")
            .and_then(|v| v.as_object_mut())
        {
            normalize_prompt_field(tg, "prompt", PromptKind::TitleGeneration, inline_prompts);
        }
        if let Some(stt) = surfaces
            .get_mut("speech_to_text")
            .and_then(|v| v.as_object_mut())
        {
            normalize_optional_prompt_field(
                stt,
                "prompt",
                PromptKind::SpeechToText,
                inline_prompts,
            );
        }
    }
}

fn normalize_prompt_field(
    obj: &mut serde_json::Map<String, Value>,
    key: &str,
    kind: PromptKind,
    inline_prompts: &mut Vec<InlinePromptWrite>,
) {
    let default_path = kind.default_path();
    match obj.remove(key) {
        Some(Value::String(s)) if !s.trim().is_empty() => {
            if looks_like_path(&s) {
                obj.insert(key.to_string(), Value::String(s));
            } else {
                inline_prompts.push(InlinePromptWrite {
                    path: default_path.to_string(),
                    content: s,
                });
                obj.insert(key.to_string(), Value::String(default_path.to_string()));
            }
        }
        _ => {
            obj.insert(key.to_string(), Value::String(default_path.to_string()));
        }
    }
}

fn normalize_optional_prompt_field(
    obj: &mut serde_json::Map<String, Value>,
    key: &str,
    kind: PromptKind,
    inline_prompts: &mut Vec<InlinePromptWrite>,
) {
    let default_path = kind.default_path();
    match obj.remove(key) {
        Some(Value::Null) | None => {
            obj.insert(key.to_string(), Value::String(default_path.to_string()));
        }
        Some(Value::String(s)) if s.trim().is_empty() => {
            obj.insert(key.to_string(), Value::String(default_path.to_string()));
        }
        Some(Value::String(s)) => {
            if looks_like_path(&s) {
                obj.insert(key.to_string(), Value::String(s));
            } else {
                inline_prompts.push(InlinePromptWrite {
                    path: default_path.to_string(),
                    content: s,
                });
                obj.insert(key.to_string(), Value::String(default_path.to_string()));
            }
        }
        Some(other) => {
            obj.insert(key.to_string(), other);
        }
    }
}

fn extract_model_level_fields(
    obj: &serde_json::Map<String, Value>,
    stt_model_id: Option<&Value>,
) -> (Option<Value>, Option<Value>, Option<Value>, Option<Value>) {
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

fn merge_into_surface<F>(surfaces: &mut serde_json::Map<String, Value>, key: &str, mut apply: F)
where
    F: FnMut(&mut serde_json::Map<String, Value>),
{
    let entry = surfaces
        .entry(key.to_string())
        .or_insert_with(|| Value::Object(serde_json::Map::new()));
    if let Some(obj) = entry.as_object_mut() {
        apply(obj);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::settings::{ModelConfig, ModelType, Provider, Settings};

    fn text_model_no_params(id: &str) -> ModelConfig {
        ModelConfig {
            id: id.into(),
            model: "gpt-4".into(),
            display_name: "T".into(),
            model_type: ModelType::Text,
            provider: Some(Provider::default()),
            group: None,
            api_key: Some("${KEY}".into()),
            base_url: None,
            parameters: None,
            context_window_size: None,
            api_mode: None,
            capabilities: None,
            store: true,
        }
    }

    #[test]
    fn migrate_model_params_fills_text_models() {
        let mut settings = Settings {
            models: vec![text_model_no_params("m1")],
            ..Default::default()
        };
        migrate_model_params(&mut settings);
        assert!(settings.models[0].parameters.is_some());
    }

    #[test]
    fn sanitize_capabilities_zeroes_invalid_effort_empty() {
        use crate::models::capabilities::{ModelCapabilities, ReasoningMode};
        let mut model = text_model_no_params("m1");
        model.capabilities = Some(ModelCapabilities {
            reasoning: ReasoningMode::Effort { allowed: vec![] },
        });
        let mut settings = Settings {
            models: vec![model],
            ..Default::default()
        };
        sanitize_capabilities(&mut settings);
        assert!(settings.models[0].capabilities.is_none());
    }

    #[test]
    fn sanitize_capabilities_zeroes_invalid_budget_min_gt_max() {
        use crate::models::capabilities::{ModelCapabilities, ReasoningMode};
        let mut model = text_model_no_params("m1");
        model.capabilities = Some(ModelCapabilities {
            reasoning: ReasoningMode::BudgetTokens { min: 100_000, max: 1024 },
        });
        let mut settings = Settings {
            models: vec![model],
            ..Default::default()
        };
        sanitize_capabilities(&mut settings);
        assert!(settings.models[0].capabilities.is_none());
    }

    #[test]
    fn migrate_explicit_capabilities_fills_known_openai_models() {
        let mut model = text_model_no_params("m1");
        model.model = "gpt-5.4".into();
        model.provider = Some(Provider::Openai);
        let mut settings = Settings {
            models: vec![model],
            ..Default::default()
        };
        migrate_explicit_capabilities(&mut settings);
        let caps = settings.models[0].capabilities.as_ref().expect("filled");
        match &caps.reasoning {
            crate::models::capabilities::ReasoningMode::Effort { allowed } => {
                assert_eq!(allowed.len(), 6);
            }
            other => panic!("expected Effort, got {other:?}"),
        }
    }

    #[test]
    fn migrate_explicit_capabilities_skips_unknown_openai_models() {
        let mut model = text_model_no_params("m1");
        model.model = "MiniMax-M2-AWQ".into();
        model.provider = Some(Provider::Openai);
        let mut settings = Settings {
            models: vec![model],
            ..Default::default()
        };
        migrate_explicit_capabilities(&mut settings);
        assert!(settings.models[0].capabilities.is_none());
    }

    #[test]
    fn migrate_explicit_capabilities_is_idempotent() {
        use crate::models::capabilities::{Effort, ModelCapabilities, ReasoningMode};
        let mut model = text_model_no_params("m1");
        model.model = "gpt-5.4".into();
        model.provider = Some(Provider::Openai);
        model.capabilities = Some(ModelCapabilities {
            reasoning: ReasoningMode::Effort {
                allowed: vec![Effort::Medium, Effort::High],
            },
        });
        let mut settings = Settings {
            models: vec![model],
            ..Default::default()
        };
        migrate_explicit_capabilities(&mut settings);
        match &settings.models[0].capabilities.as_ref().unwrap().reasoning {
            crate::models::capabilities::ReasoningMode::Effort { allowed } => {
                assert_eq!(allowed.len(), 2);
            }
            other => panic!("expected Effort, got {other:?}"),
        }
    }

    #[test]
    fn migrate_explicit_capabilities_skips_non_openai_providers() {
        let mut model = text_model_no_params("m1");
        model.model = "gpt-5.4".into();
        model.provider = Some(Provider::Anthropic);
        let mut settings = Settings {
            models: vec![model],
            ..Default::default()
        };
        migrate_explicit_capabilities(&mut settings);
        assert!(settings.models[0].capabilities.is_none());
    }

    #[test]
    fn sanitize_capabilities_keeps_valid_declarations() {
        use crate::models::capabilities::{Effort, ModelCapabilities, ReasoningMode};
        let mut model = text_model_no_params("m1");
        model.capabilities = Some(ModelCapabilities {
            reasoning: ReasoningMode::Effort {
                allowed: vec![Effort::None, Effort::Medium, Effort::High],
            },
        });
        let mut settings = Settings {
            models: vec![model],
            ..Default::default()
        };
        sanitize_capabilities(&mut settings);
        assert!(settings.models[0].capabilities.is_some());
    }

    #[test]
    fn has_legacy_fields_detects_default_model() {
        let mut obj = serde_json::Map::new();
        obj.insert("default_model".into(), "m1".into());
        assert!(has_legacy_fields(&obj));
    }

    #[test]
    fn has_legacy_fields_detects_description_generator() {
        let mut obj = serde_json::Map::new();
        obj.insert("description_generator".into(), serde_json::json!({}));
        assert!(has_legacy_fields(&obj));
    }

    #[test]
    fn has_legacy_fields_returns_false_when_clean() {
        let obj = serde_json::Map::new();
        assert!(!has_legacy_fields(&obj));
    }

    #[test]
    fn has_chat_prompt_base_fields_detects_inline() {
        let json: Value = serde_json::json!({
            "surfaces": {
                "chat": { "system_prompt": "x" }
            }
        });
        assert!(has_chat_prompt_base_fields(json.as_object().unwrap()));
    }

    #[test]
    fn has_chat_prompt_base_fields_false_when_missing() {
        let json: Value = serde_json::json!({
            "surfaces": { "chat": { "generation": {} } }
        });
        assert!(!has_chat_prompt_base_fields(json.as_object().unwrap()));
    }

    #[test]
    fn looks_like_path_md_extension() {
        assert!(looks_like_path("prompts/base/system.md"));
        assert!(looks_like_path("file.MD"));
        assert!(looks_like_path("a.markdown"));
    }

    #[test]
    fn looks_like_path_env_ref() {
        assert!(looks_like_path("${STT_PROMPT_FILE}"));
    }

    #[test]
    fn looks_like_path_rejects_inline() {
        assert!(!looks_like_path("You are a helpful assistant."));
        assert!(!looks_like_path("Multi\nline\ntext"));
        assert!(!looks_like_path("plain text without extension"));
        assert!(!looks_like_path(""));
    }

    #[test]
    fn extract_model_level_fields_pulls_stt_locale() {
        let json: Value = serde_json::json!({
            "models": [
                {
                    "id": "stt-1",
                    "type": "stt",
                    "language": "pl",
                    "keyterms_file": "kt.txt",
                    "no_verbatim": true
                }
            ]
        });
        let stt_id = Value::String("stt-1".into());
        let (lang, kt, nv, _) = extract_model_level_fields(json.as_object().unwrap(), Some(&stt_id));
        assert_eq!(lang, Some(Value::String("pl".into())));
        assert_eq!(kt, Some(Value::String("kt.txt".into())));
        assert_eq!(nv, Some(Value::Bool(true)));
    }

    #[test]
    fn parse_and_migrate_round_trips_modern_schema() {
        let modern = r#"{
            "surfaces": {
                "chat": { "generation": { "model_id": "m1", "parameters": {}, "enabled_tools": [] } },
                "speech_to_text": {}
            },
            "models": [
                { "id": "m1", "type": "text", "model": "gpt-4", "display_name": "T", "provider": "openai", "api_key": "${KEY}" }
            ]
        }"#;
        let outcome = parse_and_migrate(modern).unwrap();
        let settings = outcome.settings;
        assert_eq!(settings.models.len(), 1);
        assert_eq!(
            settings.surfaces.chat.generation.model_id.as_deref(),
            Some("m1")
        );
        assert_eq!(settings.prompt_base.system, "prompts/base/system.md");
        assert_eq!(settings.prompt_base.input_format, "prompts/base/input_format.md");
    }

    #[test]
    fn migrates_inline_system_prompt_to_file() {
        let json = r#"{
            "prompt_base": { "system_prompt": "You are a custom assistant." },
            "surfaces": {
                "chat": { "generation": { "model_id": "m1" } },
                "speech_to_text": {}
            },
            "models": [
                { "id": "m1", "type": "text", "model": "gpt-4", "display_name": "T", "provider": "openai", "api_key": "${KEY}" }
            ]
        }"#;
        let outcome = parse_and_migrate(json).unwrap();
        assert_eq!(outcome.settings.prompt_base.system, "prompts/base/system.md");
        assert_eq!(outcome.inline_prompts.len(), 1);
        assert_eq!(outcome.inline_prompts[0].path, "prompts/base/system.md");
        assert_eq!(outcome.inline_prompts[0].content, "You are a custom assistant.");
    }

    #[test]
    fn migrates_legacy_environment_section_field_name() {
        let json = r#"{
            "prompt_base": { "environment_section": "environment_section.md" },
            "surfaces": {
                "chat": { "generation": { "model_id": "m1" } },
                "speech_to_text": {}
            },
            "models": [
                { "id": "m1", "type": "text", "model": "gpt-4", "display_name": "T", "provider": "openai", "api_key": "${KEY}" }
            ]
        }"#;
        let outcome = parse_and_migrate(json).unwrap();
        assert_eq!(outcome.settings.prompt_base.environment, "environment_section.md");
    }

    #[test]
    fn migrates_inline_title_generation_prompt() {
        let json = r#"{
            "prompt_base": {},
            "surfaces": {
                "chat": { "generation": { "model_id": "m1" } },
                "title_generation": { "prompt": "Custom title prompt" },
                "speech_to_text": {}
            },
            "models": [
                { "id": "m1", "type": "text", "model": "gpt-4", "display_name": "T", "provider": "openai", "api_key": "${KEY}" }
            ]
        }"#;
        let outcome = parse_and_migrate(json).unwrap();
        assert_eq!(
            outcome.settings.surfaces.title_generation.prompt,
            "prompts/surfaces/title_generation.md"
        );
        assert_eq!(outcome.inline_prompts.len(), 1);
        assert_eq!(
            outcome.inline_prompts[0].path,
            "prompts/surfaces/title_generation.md"
        );
        assert_eq!(outcome.inline_prompts[0].content, "Custom title prompt");
    }

    #[test]
    fn preserves_path_like_prompt_value() {
        let json = r#"{
            "prompt_base": { "system": "prompts/base/system.md", "about_me": "prompts/base/about_me.md", "environment": "prompts/base/environment.md", "input_format": "prompts/base/input_format.md" },
            "surfaces": {
                "chat": { "generation": { "model_id": "m1" } },
                "title_generation": { "prompt": "prompts/surfaces/title_generation.md" },
                "speech_to_text": { "prompt": "prompts/surfaces/speech_to_text.md" }
            },
            "models": [
                { "id": "m1", "type": "text", "model": "gpt-4", "display_name": "T", "provider": "openai", "api_key": "${KEY}" }
            ]
        }"#;
        let outcome = parse_and_migrate(json).unwrap();
        assert!(outcome.inline_prompts.is_empty());
        assert_eq!(outcome.settings.prompt_base.system, "prompts/base/system.md");
    }

    #[test]
    fn drops_description_generator() {
        let json = r#"{
            "description_generator": { "model": "x", "prompt": "old" },
            "prompt_base": {},
            "surfaces": {
                "chat": { "generation": { "model_id": "m1" } },
                "speech_to_text": {}
            },
            "models": [
                { "id": "m1", "type": "text", "model": "gpt-4", "display_name": "T", "provider": "openai", "api_key": "${KEY}" }
            ]
        }"#;
        let outcome = parse_and_migrate(json).unwrap();
        let json_back = serde_json::to_value(&outcome.settings).unwrap();
        assert!(json_back.get("description_generator").is_none());
    }

    #[test]
    fn empty_speech_to_text_prompt_defaults_to_canonical_path() {
        let json = r#"{
            "prompt_base": {},
            "surfaces": {
                "chat": { "generation": { "model_id": "m1" } },
                "speech_to_text": { "prompt": "" }
            },
            "models": [
                { "id": "m1", "type": "text", "model": "gpt-4", "display_name": "T", "provider": "openai", "api_key": "${KEY}" }
            ]
        }"#;
        let outcome = parse_and_migrate(json).unwrap();
        assert_eq!(
            outcome.settings.surfaces.speech_to_text.prompt.as_deref(),
            Some("prompts/surfaces/speech_to_text.md"),
        );
    }

    #[test]
    fn env_ref_path_preserved_for_speech_to_text() {
        let json = r#"{
            "prompt_base": {},
            "surfaces": {
                "chat": { "generation": { "model_id": "m1" } },
                "speech_to_text": { "prompt": "${STT_PROMPT_FILE}" }
            },
            "models": [
                { "id": "m1", "type": "text", "model": "gpt-4", "display_name": "T", "provider": "openai", "api_key": "${KEY}" }
            ]
        }"#;
        let outcome = parse_and_migrate(json).unwrap();
        assert_eq!(
            outcome.settings.surfaces.speech_to_text.prompt.as_deref(),
            Some("${STT_PROMPT_FILE}"),
        );
        assert!(outcome.inline_prompts.is_empty());
    }

    #[test]
    fn missing_input_format_gets_default_path() {
        let json = r#"{
            "prompt_base": {
                "system": "prompts/base/system.md",
                "about_me": "prompts/base/about_me.md",
                "environment": "prompts/base/environment.md"
            },
            "surfaces": {
                "chat": { "generation": { "model_id": "m1" } },
                "title_generation": { "prompt": "prompts/surfaces/title_generation.md" },
                "speech_to_text": { "prompt": "prompts/surfaces/speech_to_text.md" }
            },
            "models": [
                { "id": "m1", "type": "text", "model": "gpt-4", "display_name": "T", "provider": "openai", "api_key": "${KEY}" }
            ]
        }"#;
        let outcome = parse_and_migrate(json).unwrap();
        assert_eq!(
            outcome.settings.prompt_base.input_format,
            "prompts/base/input_format.md"
        );
    }
}
