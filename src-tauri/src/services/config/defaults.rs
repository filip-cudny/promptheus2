use crate::models::settings::Settings;

use super::ConfigError;

pub(super) fn ensure_surface_defaults(settings: &mut Settings) {
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

pub(super) fn ensure_generation(
    surface: &mut serde_json::Map<String, serde_json::Value>,
) -> &mut serde_json::Map<String, serde_json::Value> {
    let entry = surface
        .entry("generation".to_string())
        .or_insert_with(|| serde_json::Value::Object(serde_json::Map::new()));
    entry.as_object_mut().expect("generation must be object")
}

pub(super) fn ensure_parameters(
    gen: &mut serde_json::Map<String, serde_json::Value>,
) -> &mut serde_json::Map<String, serde_json::Value> {
    let entry = gen
        .entry("parameters".to_string())
        .or_insert_with(|| serde_json::Value::Object(serde_json::Map::new()));
    entry.as_object_mut().expect("parameters must be object")
}

pub(super) fn validate(settings: &Settings) -> Result<(), ConfigError> {
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::settings::{ModelConfig, ModelType, Provider, Settings, WebviewProvider};

    fn text_model(id: &str) -> ModelConfig {
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
    fn ensure_surface_defaults_picks_text_fallback() {
        let mut settings = Settings {
            models: vec![text_model("m1")],
            ..Default::default()
        };
        ensure_surface_defaults(&mut settings);
        assert_eq!(
            settings.surfaces.chat.generation.model_id.as_deref(),
            Some("m1")
        );
        assert_eq!(
            settings.surfaces.quick_actions.generation.model_id.as_deref(),
            Some("m1")
        );
        assert_eq!(
            settings.surfaces.title_generation.generation.model_id.as_deref(),
            Some("m1")
        );
    }

    #[test]
    fn ensure_surface_defaults_drops_unknown_chat_model() {
        let mut settings = Settings {
            models: vec![text_model("real")],
            ..Default::default()
        };
        settings.surfaces.chat.generation.model_id = Some("ghost".into());
        ensure_surface_defaults(&mut settings);
        assert_eq!(
            settings.surfaces.chat.generation.model_id.as_deref(),
            Some("real")
        );
    }

    #[test]
    fn ensure_generation_creates_object_when_missing() {
        let mut surface = serde_json::Map::new();
        let gen = ensure_generation(&mut surface);
        assert!(gen.is_empty());
        assert!(surface.get("generation").unwrap().is_object());
    }

    #[test]
    fn ensure_parameters_creates_object_when_missing() {
        let mut gen = serde_json::Map::new();
        let params = ensure_parameters(&mut gen);
        assert!(params.is_empty());
        assert!(gen.get("parameters").unwrap().is_object());
    }

    #[test]
    fn validate_no_models_fails() {
        let mut settings = Settings::default();
        settings.models.clear();
        assert!(validate(&settings).is_err());
    }

    #[test]
    fn validate_duplicate_id_fails() {
        let m = text_model("dup");
        let settings = Settings {
            models: vec![m.clone(), m],
            ..Default::default()
        };
        let err = validate(&settings).unwrap_err().to_string();
        assert!(err.contains("Duplicate model ID"), "{err}");
    }

    #[test]
    fn validate_webview_providers_invalid_scheme_fails() {
        let settings = Settings {
            models: vec![text_model("1")],
            webview_providers: vec![WebviewProvider {
                id: "ftp".into(),
                name: "Ftp".into(),
                url: "ftp://x.example".into(),
            }],
            ..Default::default()
        };
        let err = validate(&settings).unwrap_err().to_string();
        assert!(err.contains("http or https"), "{err}");
    }

    #[test]
    fn validate_webview_providers_duplicate_id_fails() {
        let settings = Settings {
            models: vec![text_model("1")],
            webview_providers: vec![
                WebviewProvider {
                    id: "claude".into(),
                    name: "Claude".into(),
                    url: "https://claude.ai/".into(),
                },
                WebviewProvider {
                    id: "claude".into(),
                    name: "Claude2".into(),
                    url: "https://claude.ai/".into(),
                },
            ],
            ..Default::default()
        };
        let err = validate(&settings).unwrap_err().to_string();
        assert!(err.contains("Duplicate webview_providers"), "{err}");
    }
}
