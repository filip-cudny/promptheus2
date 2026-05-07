pub use crate::models::capabilities::{Effort, ModelCapabilities, ReasoningMode};

use crate::models::settings::{ModelConfig, Provider};

pub fn resolve(model: &ModelConfig) -> ModelCapabilities {
    model.capabilities.clone().unwrap_or_else(|| infer_default(model))
}

fn infer_default(model: &ModelConfig) -> ModelCapabilities {
    let provider = model.provider.clone().unwrap_or_default();
    capabilities_for(&provider, &model.model)
}

pub fn capabilities_for(provider: &Provider, model: &str) -> ModelCapabilities {
    match provider {
        Provider::Openai => openai_capabilities(model),
        Provider::Anthropic => anthropic_capabilities(model),
        Provider::Gemini => gemini_capabilities(model),
        Provider::ElevenLabs => ModelCapabilities::minimal(),
    }
}

const OPENAI_EFFORT_MODELS: &[&str] = &[
    "o1",
    "o1-mini",
    "o1-preview",
    "o3",
    "o3-mini",
    "o3-pro",
    "o3-deep-research",
    "o4-mini",
    "o4-mini-deep-research",
    "gpt-5",
    "gpt-5-mini",
    "gpt-5-nano",
    "gpt-5.3-codex",
    "gpt-5.4",
    "gpt-5.4-mini",
    "gpt-5.4-nano",
    "gpt-5.4-pro",
];

fn openai_capabilities(model: &str) -> ModelCapabilities {
    let reasoning = if OPENAI_EFFORT_MODELS.contains(&model) {
        ReasoningMode::Effort {
            allowed: vec![
                Effort::None,
                Effort::Minimal,
                Effort::Low,
                Effort::Medium,
                Effort::High,
                Effort::XHigh,
            ],
        }
    } else {
        ReasoningMode::Unsupported
    };

    ModelCapabilities { reasoning }
}

fn anthropic_capabilities(_model: &str) -> ModelCapabilities {
    ModelCapabilities::minimal()
}

fn gemini_capabilities(_model: &str) -> ModelCapabilities {
    ModelCapabilities::minimal()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn openai_gpt5_supports_effort_full_range() {
        let caps = capabilities_for(&Provider::Openai, "gpt-5");
        let ReasoningMode::Effort { ref allowed } = caps.reasoning else {
            panic!("expected Effort");
        };
        assert_eq!(
            *allowed,
            vec![
                Effort::None,
                Effort::Minimal,
                Effort::Low,
                Effort::Medium,
                Effort::High,
                Effort::XHigh,
            ]
        );
        assert!(caps.accepts_effort("medium"));
        assert!(caps.accepts_effort("xhigh"));
        assert!(caps.accepts_effort("minimal"));
    }

    #[test]
    fn openai_minimax_is_unsupported() {
        let caps = capabilities_for(&Provider::Openai, "MiniMax-M2-AWQ");
        assert!(matches!(caps.reasoning, ReasoningMode::Unsupported));
        assert!(!caps.accepts_effort("medium"));
    }

    #[test]
    fn unknown_openai_model_defaults_to_unsupported() {
        let caps = capabilities_for(&Provider::Openai, "some-custom-model");
        assert!(matches!(caps.reasoning, ReasoningMode::Unsupported));
    }

    #[test]
    fn unknown_effort_value_rejected() {
        let caps = capabilities_for(&Provider::Openai, "gpt-5");
        assert!(!caps.accepts_effort("extreme"));
    }

    fn make_model(id: &str, model: &str, provider: Option<Provider>) -> ModelConfig {
        ModelConfig {
            id: id.into(),
            model: model.into(),
            display_name: id.into(),
            model_type: crate::models::settings::ModelType::Text,
            provider,
            group: None,
            api_key: None,
            base_url: None,
            parameters: None,
            context_window_size: None,
            api_mode: None,
            capabilities: None,
            store: true,
        }
    }

    #[test]
    fn resolve_uses_explicit_capabilities_when_present() {
        let mut config = make_model("m", "MiniMax-M2-AWQ", Some(Provider::Openai));
        config.capabilities = Some(ModelCapabilities {
            reasoning: ReasoningMode::Effort {
                allowed: vec![Effort::Medium, Effort::High],
            },
        });
        let caps = resolve(&config);
        assert!(matches!(caps.reasoning, ReasoningMode::Effort { .. }));
        assert!(caps.accepts_effort("medium"));
        assert!(!caps.accepts_effort("low"));
    }

    #[test]
    fn resolve_falls_back_to_inferred_default() {
        let config = make_model("m", "gpt-5", Some(Provider::Openai));
        let caps = resolve(&config);
        assert!(matches!(caps.reasoning, ReasoningMode::Effort { .. }));
        assert!(caps.accepts_effort("xhigh"));
    }

    #[test]
    fn resolve_inferred_for_unknown_model_is_unsupported() {
        let config = make_model("m", "MiniMax-M2-AWQ", Some(Provider::Openai));
        let caps = resolve(&config);
        assert!(matches!(caps.reasoning, ReasoningMode::Unsupported));
    }
}
