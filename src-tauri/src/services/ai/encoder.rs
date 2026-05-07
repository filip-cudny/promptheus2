use serde_json::{Map, Value};

use crate::models::capabilities::{ModelCapabilities, ReasoningMode};
use crate::models::settings::ModelParameters;

pub trait CapabilityEncoder: Send + Sync {
    fn encode_reasoning(
        &self,
        caps: &ModelCapabilities,
        params: &ModelParameters,
        out: &mut Map<String, Value>,
    );
}

pub struct OpenAiCompletionsEncoder;

impl CapabilityEncoder for OpenAiCompletionsEncoder {
    fn encode_reasoning(
        &self,
        caps: &ModelCapabilities,
        params: &ModelParameters,
        out: &mut Map<String, Value>,
    ) {
        let Some(value) = params.reasoning_effort.as_deref() else {
            return;
        };
        match &caps.reasoning {
            ReasoningMode::Effort { .. } if caps.accepts_effort(value) => {
                out.insert("reasoning_effort".into(), Value::String(value.into()));
            }
            ReasoningMode::Toggle => {}
            ReasoningMode::BudgetTokens { .. } => {}
            _ => {}
        }
    }
}

pub struct OpenAiResponsesEncoder;

impl CapabilityEncoder for OpenAiResponsesEncoder {
    fn encode_reasoning(
        &self,
        caps: &ModelCapabilities,
        params: &ModelParameters,
        out: &mut Map<String, Value>,
    ) {
        let Some(value) = params.reasoning_effort.as_deref() else {
            return;
        };
        if !matches!(caps.reasoning, ReasoningMode::Effort { .. }) {
            return;
        }
        if !caps.accepts_effort(value) {
            return;
        }
        let mut reasoning = serde_json::json!({ "effort": value });
        if value != "none" {
            reasoning["summary"] = Value::String("auto".into());
        }
        out.insert("reasoning".into(), reasoning);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::capabilities::Effort;

    fn caps_effort(allowed: Vec<Effort>) -> ModelCapabilities {
        ModelCapabilities {
            reasoning: ReasoningMode::Effort { allowed },
        }
    }

    fn params_with_effort(value: &str) -> ModelParameters {
        ModelParameters {
            reasoning_effort: Some(value.into()),
            ..Default::default()
        }
    }

    #[test]
    fn completions_encoder_passes_accepted_effort() {
        let mut out = Map::new();
        OpenAiCompletionsEncoder.encode_reasoning(
            &caps_effort(vec![Effort::Medium, Effort::High]),
            &params_with_effort("medium"),
            &mut out,
        );
        assert_eq!(out.get("reasoning_effort"), Some(&Value::String("medium".into())));
    }

    #[test]
    fn completions_encoder_passes_implicit_none() {
        let mut out = Map::new();
        OpenAiCompletionsEncoder.encode_reasoning(
            &caps_effort(vec![Effort::Medium]),
            &params_with_effort("none"),
            &mut out,
        );
        assert_eq!(out.get("reasoning_effort"), Some(&Value::String("none".into())));
    }

    #[test]
    fn completions_encoder_drops_disallowed_effort() {
        let mut out = Map::new();
        OpenAiCompletionsEncoder.encode_reasoning(
            &caps_effort(vec![Effort::Medium]),
            &params_with_effort("xhigh"),
            &mut out,
        );
        assert!(out.is_empty());
    }

    #[test]
    fn completions_encoder_does_nothing_when_unsupported() {
        let mut out = Map::new();
        OpenAiCompletionsEncoder.encode_reasoning(
            &ModelCapabilities::minimal(),
            &params_with_effort("medium"),
            &mut out,
        );
        assert!(out.is_empty());
    }

    #[test]
    fn responses_encoder_includes_summary_for_non_none() {
        let mut out = Map::new();
        OpenAiResponsesEncoder.encode_reasoning(
            &caps_effort(vec![Effort::Medium]),
            &params_with_effort("medium"),
            &mut out,
        );
        let reasoning = out.get("reasoning").and_then(|v| v.as_object()).unwrap();
        assert_eq!(reasoning.get("effort").and_then(|v| v.as_str()), Some("medium"));
        assert_eq!(reasoning.get("summary").and_then(|v| v.as_str()), Some("auto"));
    }

    #[test]
    fn responses_encoder_omits_summary_for_none() {
        let mut out = Map::new();
        OpenAiResponsesEncoder.encode_reasoning(
            &caps_effort(vec![Effort::Medium]),
            &params_with_effort("none"),
            &mut out,
        );
        let reasoning = out.get("reasoning").and_then(|v| v.as_object()).unwrap();
        assert_eq!(reasoning.get("effort").and_then(|v| v.as_str()), Some("none"));
        assert!(reasoning.get("summary").is_none());
    }

    #[test]
    fn responses_encoder_drops_disallowed_effort() {
        let mut out = Map::new();
        OpenAiResponsesEncoder.encode_reasoning(
            &caps_effort(vec![Effort::Medium]),
            &params_with_effort("xhigh"),
            &mut out,
        );
        assert!(out.is_empty());
    }

    #[test]
    fn responses_encoder_does_nothing_when_unsupported() {
        let mut out = Map::new();
        OpenAiResponsesEncoder.encode_reasoning(
            &ModelCapabilities::minimal(),
            &params_with_effort("medium"),
            &mut out,
        );
        assert!(out.is_empty());
    }
}
