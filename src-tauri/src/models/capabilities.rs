use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Effort {
    None,
    Minimal,
    Low,
    Medium,
    High,
    #[serde(rename = "xhigh")]
    XHigh,
}

impl Effort {
    pub fn as_str(self) -> &'static str {
        match self {
            Effort::None => "none",
            Effort::Minimal => "minimal",
            Effort::Low => "low",
            Effort::Medium => "medium",
            Effort::High => "high",
            Effort::XHigh => "xhigh",
        }
    }

    pub fn parse(value: &str) -> Option<Self> {
        match value {
            "none" => Some(Effort::None),
            "minimal" => Some(Effort::Minimal),
            "low" => Some(Effort::Low),
            "medium" => Some(Effort::Medium),
            "high" => Some(Effort::High),
            "xhigh" => Some(Effort::XHigh),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum ReasoningMode {
    Unsupported,
    Effort { allowed: Vec<Effort> },
    BudgetTokens { min: u32, max: u32 },
    Toggle,
}

impl Default for ReasoningMode {
    fn default() -> Self {
        ReasoningMode::Unsupported
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq, Eq)]
pub struct ModelCapabilities {
    #[serde(default)]
    pub reasoning: ReasoningMode,
}

impl ModelCapabilities {
    pub fn minimal() -> Self {
        Self::default()
    }

    pub fn accepts_effort(&self, value: &str) -> bool {
        match &self.reasoning {
            ReasoningMode::Effort { allowed } => {
                let Some(parsed) = Effort::parse(value) else {
                    return false;
                };
                parsed == Effort::None || allowed.contains(&parsed)
            }
            _ => false,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn effort_roundtrip_all_variants() {
        for v in [
            Effort::None,
            Effort::Minimal,
            Effort::Low,
            Effort::Medium,
            Effort::High,
            Effort::XHigh,
        ] {
            let serialized = serde_json::to_string(&v).unwrap();
            let parsed: Effort = serde_json::from_str(&serialized).unwrap();
            assert_eq!(v, parsed);
            assert_eq!(Effort::parse(v.as_str()), Some(v));
        }
    }

    #[test]
    fn effort_xhigh_uses_lowercase_string() {
        let s = serde_json::to_string(&Effort::XHigh).unwrap();
        assert_eq!(s, "\"xhigh\"");
    }

    #[test]
    fn reasoning_mode_unsupported_default() {
        let m = ReasoningMode::default();
        assert!(matches!(m, ReasoningMode::Unsupported));
    }

    #[test]
    fn reasoning_mode_effort_serde_roundtrip() {
        let m = ReasoningMode::Effort {
            allowed: vec![Effort::Low, Effort::Medium, Effort::High],
        };
        let s = serde_json::to_string(&m).unwrap();
        let back: ReasoningMode = serde_json::from_str(&s).unwrap();
        assert_eq!(m, back);
    }

    #[test]
    fn reasoning_mode_budget_tokens_roundtrip() {
        let m = ReasoningMode::BudgetTokens {
            min: 1024,
            max: 64_000,
        };
        let s = serde_json::to_string(&m).unwrap();
        let back: ReasoningMode = serde_json::from_str(&s).unwrap();
        assert_eq!(m, back);
    }

    #[test]
    fn accepts_effort_implicit_none() {
        let caps = ModelCapabilities {
            reasoning: ReasoningMode::Effort {
                allowed: vec![Effort::Medium, Effort::High],
            },
        };
        assert!(caps.accepts_effort("none"));
        assert!(caps.accepts_effort("medium"));
        assert!(!caps.accepts_effort("low"));
        assert!(!caps.accepts_effort("bogus"));
    }

    #[test]
    fn accepts_effort_rejects_when_unsupported() {
        let caps = ModelCapabilities::minimal();
        assert!(!caps.accepts_effort("none"));
        assert!(!caps.accepts_effort("medium"));
    }
}
