use crate::models::settings::Settings;

use super::defaults::{ensure_generation, ensure_parameters};
use super::ConfigError;

pub(super) fn parse_and_migrate(content: &str) -> Result<Settings, ConfigError> {
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

pub(super) fn migrate_model_params(settings: &mut Settings) {
    for model in &mut settings.models {
        if model.is_text() && model.parameters.is_none() {
            model.parameters = Some(Default::default());
        }
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

    obj.insert("surfaces".to_string(), serde_json::Value::Object(surfaces));

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
    fn has_legacy_fields_detects_default_model() {
        let mut obj = serde_json::Map::new();
        obj.insert("default_model".into(), "m1".into());
        assert!(has_legacy_fields(&obj));
    }

    #[test]
    fn has_legacy_fields_returns_false_when_clean() {
        let obj = serde_json::Map::new();
        assert!(!has_legacy_fields(&obj));
    }

    #[test]
    fn has_chat_prompt_base_fields_detects_inline() {
        let json: serde_json::Value = serde_json::json!({
            "surfaces": {
                "chat": { "system_prompt": "x" }
            }
        });
        assert!(has_chat_prompt_base_fields(json.as_object().unwrap()));
    }

    #[test]
    fn has_chat_prompt_base_fields_false_when_missing() {
        let json: serde_json::Value = serde_json::json!({
            "surfaces": { "chat": { "generation": {} } }
        });
        assert!(!has_chat_prompt_base_fields(json.as_object().unwrap()));
    }

    #[test]
    fn extract_model_level_fields_pulls_stt_locale() {
        let json: serde_json::Value = serde_json::json!({
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
        let stt_id = serde_json::Value::String("stt-1".into());
        let (lang, kt, nv, _) = extract_model_level_fields(json.as_object().unwrap(), Some(&stt_id));
        assert_eq!(lang, Some(serde_json::Value::String("pl".into())));
        assert_eq!(kt, Some(serde_json::Value::String("kt.txt".into())));
        assert_eq!(nv, Some(serde_json::Value::Bool(true)));
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
        let settings = parse_and_migrate(modern).unwrap();
        assert_eq!(settings.models.len(), 1);
        assert_eq!(
            settings.surfaces.chat.generation.model_id.as_deref(),
            Some("m1")
        );
    }
}
