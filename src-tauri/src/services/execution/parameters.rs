use crate::models::settings::ModelParameters;

pub fn merge_optional_parameters(
    base: Option<ModelParameters>,
    overrides: Option<ModelParameters>,
) -> Option<ModelParameters> {
    match (base, overrides) {
        (None, None) => None,
        (Some(b), None) => Some(b),
        (None, Some(o)) => Some(o),
        (Some(b), Some(o)) => {
            let mut extra = b.extra.clone();
            extra.extend(o.extra.clone());
            Some(ModelParameters {
                temperature: o.temperature.or(b.temperature),
                max_tokens: o.max_tokens.or(b.max_tokens),
                top_p: o.top_p.or(b.top_p),
                frequency_penalty: o.frequency_penalty.or(b.frequency_penalty),
                presence_penalty: o.presence_penalty.or(b.presence_penalty),
                reasoning_effort: o.reasoning_effort.or(b.reasoning_effort),
                extra,
            })
        }
    }
}

pub fn surface_effort_override(
    base: Option<String>,
    override_effort: Option<String>,
) -> Option<ModelParameters> {
    let effort = override_effort.or(base);
    effort.map(|e| ModelParameters {
        reasoning_effort: Some(e),
        ..Default::default()
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn params_with_temp(temp: f64) -> ModelParameters {
        ModelParameters {
            temperature: Some(temp),
            ..Default::default()
        }
    }

    #[test]
    fn merge_returns_none_when_both_empty() {
        assert!(merge_optional_parameters(None, None).is_none());
    }

    #[test]
    fn merge_returns_base_when_no_overrides() {
        let merged = merge_optional_parameters(Some(params_with_temp(0.5)), None).unwrap();
        assert_eq!(merged.temperature, Some(0.5));
    }

    #[test]
    fn merge_overrides_win_over_base() {
        let base = ModelParameters {
            temperature: Some(0.2),
            max_tokens: Some(100),
            ..Default::default()
        };
        let overrides = ModelParameters {
            temperature: Some(0.9),
            ..Default::default()
        };
        let merged = merge_optional_parameters(Some(base), Some(overrides)).unwrap();
        assert_eq!(merged.temperature, Some(0.9));
        assert_eq!(merged.max_tokens, Some(100));
    }

    #[test]
    fn surface_effort_override_applies_override() {
        let result = surface_effort_override(None, Some("high".into())).unwrap();
        assert_eq!(result.reasoning_effort, Some("high".into()));
    }

    #[test]
    fn surface_effort_override_falls_back_to_base() {
        let result = surface_effort_override(Some("low".into()), None).unwrap();
        assert_eq!(result.reasoning_effort, Some("low".into()));
    }

    #[test]
    fn surface_effort_override_none_when_both_empty() {
        assert!(surface_effort_override(None, None).is_none());
    }
}
