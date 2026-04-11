use regex::Regex;
use std::sync::LazyLock;

static ENV_REF_PATTERN: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"\$\{([^}]+)\}").expect("invalid env ref regex"));

pub fn resolve_env_refs(value: &str) -> String {
    ENV_REF_PATTERN
        .replace_all(value, |caps: &regex::Captures| {
            let var_name = &caps[1];
            match std::env::var(var_name) {
                Ok(val) => val,
                Err(_) => {
                    log::warn!("env variable '{}' not found during resolution", var_name);
                    String::new()
                }
            }
        })
        .into_owned()
}

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
    fn has_env_refs_detection() {
        assert!(has_env_refs("${FOO}"));
        assert!(has_env_refs("prefix_${FOO}_suffix"));
        assert!(!has_env_refs("no refs here"));
        assert!(!has_env_refs("$FOO"));
        assert!(!has_env_refs(""));
    }
}
