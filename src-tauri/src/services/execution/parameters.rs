use crate::models::settings::ModelParameters;

pub fn merge_optional_parameters(
    base: Option<ModelParameters>,
    overrides: Option<ModelParameters>,
) -> Option<ModelParameters> {
    match (base, overrides) {
        (Some(base), Some(overrides)) => Some(base.merged_with(&overrides)),
        (base, overrides) => overrides.or(base),
    }
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
}
