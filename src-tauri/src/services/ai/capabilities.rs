use serde::Serialize;

use crate::models::settings::Provider;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Effort {
    Low,
    Medium,
    High,
}

impl Effort {
    pub fn as_str(self) -> &'static str {
        match self {
            Effort::Low => "low",
            Effort::Medium => "medium",
            Effort::High => "high",
        }
    }

    pub fn parse(value: &str) -> Option<Self> {
        match value {
            "low" => Some(Effort::Low),
            "medium" => Some(Effort::Medium),
            "high" => Some(Effort::High),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum ReasoningMode {
    Unsupported,
    Effort { allowed: Vec<Effort> },
    BudgetTokens { min: u32, max: u32 },
    Toggle,
}

#[derive(Debug, Clone, Serialize)]
pub struct ModelCapabilities {
    pub reasoning: ReasoningMode,
}

impl ModelCapabilities {
    pub fn minimal() -> Self {
        Self {
            reasoning: ReasoningMode::Unsupported,
        }
    }

    pub fn accepts_effort(&self, value: &str) -> bool {
        match &self.reasoning {
            ReasoningMode::Effort { allowed } => Effort::parse(value)
                .map(|e| allowed.contains(&e))
                .unwrap_or(false),
            _ => false,
        }
    }
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
            allowed: vec![Effort::Low, Effort::Medium, Effort::High],
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
    fn openai_gpt5_supports_effort() {
        let caps = capabilities_for(&Provider::Openai, "gpt-5");
        assert!(matches!(caps.reasoning, ReasoningMode::Effort { .. }));
        assert!(caps.accepts_effort("medium"));
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
}
