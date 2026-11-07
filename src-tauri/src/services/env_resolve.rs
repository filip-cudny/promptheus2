use regex::Regex;
use std::collections::HashMap;
use std::sync::{LazyLock, RwLock};

static ENV_REF_PATTERN: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"\$\{([^}]+)\}").expect("invalid env ref regex"));

static OVERRIDES: LazyLock<RwLock<HashMap<String, Option<String>>>> =
    LazyLock::new(|| RwLock::new(HashMap::new()));

pub fn set_override(name: &str, value: Option<String>) {
    OVERRIDES
        .write()
        .unwrap_or_else(|e| e.into_inner())
        .insert(name.to_string(), value);
}

pub fn clear_override(name: &str) {
    OVERRIDES
        .write()
        .unwrap_or_else(|e| e.into_inner())
        .remove(name);
}

pub fn override_for(name: &str) -> Option<Option<String>> {
    OVERRIDES
        .read()
        .unwrap_or_else(|e| e.into_inner())
        .get(name)
        .cloned()
}

pub fn override_values() -> Vec<String> {
    OVERRIDES
        .read()
        .unwrap_or_else(|e| e.into_inner())
        .values()
        .flatten()
        .cloned()
        .collect()
}

pub fn env_value(name: &str) -> Option<String> {
    match override_for(name) {
        Some(value) => value,
        None => std::env::var(name).ok(),
    }
}

pub fn resolve_env_refs(value: &str) -> String {
    ENV_REF_PATTERN
        .replace_all(value, |caps: &regex::Captures| {
            let var_name = &caps[1];
            match env_value(var_name) {
                Some(val) => val,
                None => {
                    log::warn!("env variable '{}' not found during resolution", var_name);
                    String::new()
                }
            }
        })
        .into_owned()
}

pub fn env_ref_names(value: &str) -> Vec<String> {
    ENV_REF_PATTERN
        .captures_iter(value)
        .map(|caps| caps[1].to_string())
        .collect()
}

#[cfg(test)]
pub fn has_env_refs(value: &str) -> bool {
    ENV_REF_PATTERN.is_match(value)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn literal_value_unchanged() {
        assert_eq!(resolve_env_refs("hello world"), "hello world");
    }

    #[test]
    fn empty_string_unchanged() {
        assert_eq!(resolve_env_refs(""), "");
    }

    #[test]
    fn full_replacement() {
        std::env::set_var("TEST_ENV_RESOLVE_FULL", "resolved_value");
        assert_eq!(resolve_env_refs("${TEST_ENV_RESOLVE_FULL}"), "resolved_value");
        std::env::remove_var("TEST_ENV_RESOLVE_FULL");
    }

    #[test]
    fn interpolation() {
        std::env::set_var("TEST_ENV_RESOLVE_HOME", "/home/user");
        assert_eq!(
            resolve_env_refs("${TEST_ENV_RESOLVE_HOME}/tools/mcp/index.ts"),
            "/home/user/tools/mcp/index.ts"
        );
        std::env::remove_var("TEST_ENV_RESOLVE_HOME");
    }

    #[test]
    fn multiple_refs() {
        std::env::set_var("TEST_ENV_RESOLVE_A", "alpha");
        std::env::set_var("TEST_ENV_RESOLVE_B", "beta");
        assert_eq!(
            resolve_env_refs("${TEST_ENV_RESOLVE_A}-${TEST_ENV_RESOLVE_B}"),
            "alpha-beta"
        );
        std::env::remove_var("TEST_ENV_RESOLVE_A");
        std::env::remove_var("TEST_ENV_RESOLVE_B");
    }

    #[test]
    fn missing_var_resolves_to_empty() {
        assert_eq!(resolve_env_refs("${DEFINITELY_MISSING_VAR_XYZ}"), "");
    }

    #[test]
    fn env_ref_names_lists_every_reference() {
        assert_eq!(env_ref_names("${A}/x/${B}"), vec!["A", "B"]);
        assert!(env_ref_names("no refs $A").is_empty());
    }

    #[test]
    fn clear_override_removes_the_entry_instead_of_setting_it_to_none() {
        set_override("TEST_ENV_RESOLVE_CLEAR", Some("v".into()));
        assert_eq!(override_for("TEST_ENV_RESOLVE_CLEAR"), Some(Some("v".to_string())));

        clear_override("TEST_ENV_RESOLVE_CLEAR");

        assert_eq!(override_for("TEST_ENV_RESOLVE_CLEAR"), None);
    }

    #[test]
    fn has_env_refs_detection() {
        assert!(has_env_refs("${FOO}"));
        assert!(has_env_refs("prefix_${FOO}_suffix"));
        assert!(!has_env_refs("no refs here"));
        assert!(!has_env_refs("$FOO"));
        assert!(!has_env_refs(""));
    }
}
