use std::collections::HashMap;
use std::sync::OnceLock;

static SHELL_ENV: OnceLock<HashMap<String, String>> = OnceLock::new();

pub fn get_shell_env() -> &'static HashMap<String, String> {
    SHELL_ENV.get_or_init(|| {
        let shell = std::env::var("SHELL").unwrap_or_else(|_| "/bin/sh".to_string());

        let output = std::process::Command::new(&shell)
            .args(["-i", "-l", "-c", "env -0 2>/dev/null || env"])
            .stderr(std::process::Stdio::null())
            .output();

        match output {
            Ok(out) if out.status.success() => {
                let env = if out.stdout.contains(&0u8) {
                    parse_null_separated(&out.stdout)
                } else {
                    parse_line_based(&out.stdout)
                };
                log::info!("shell env loaded: {} variables from {}", env.len(), shell);
                env
            }
            Ok(out) => {
                log::warn!(
                    "login shell exited with {}: {}",
                    out.status,
                    String::from_utf8_lossy(&out.stderr)
                );
                HashMap::new()
            }
            Err(e) => {
                log::warn!("failed to read login shell env: {}", e);
                HashMap::new()
            }
        }
    })
}

fn parse_null_separated(data: &[u8]) -> HashMap<String, String> {
    data.split(|&b| b == 0)
        .filter_map(|entry| {
            let s = String::from_utf8_lossy(entry);
            s.split_once('=')
                .map(|(k, v)| (k.to_string(), v.to_string()))
        })
        .collect()
}

fn parse_line_based(data: &[u8]) -> HashMap<String, String> {
    let text = String::from_utf8_lossy(data);
    let mut env = HashMap::new();
    let mut current_key = String::new();
    let mut current_val = String::new();

    for line in text.lines() {
        if let Some(pos) = line.find('=') {
            let key = &line[..pos];
            if is_valid_env_key(key) {
                if !current_key.is_empty() {
                    env.insert(std::mem::take(&mut current_key), std::mem::take(&mut current_val));
                }
                current_key = key.to_string();
                current_val = line[pos + 1..].to_string();
                continue;
            }
        }
        if !current_key.is_empty() {
            current_val.push('\n');
            current_val.push_str(line);
        }
    }
    if !current_key.is_empty() {
        env.insert(current_key, current_val);
    }
    env
}

fn is_valid_env_key(s: &str) -> bool {
    !s.is_empty()
        && s.as_bytes()[0].is_ascii_alphabetic()
        && s.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'_')
}

pub fn resolve_command(command: &str, env: &HashMap<String, String>) -> String {
    if command.contains(std::path::MAIN_SEPARATOR) || command.starts_with('/') {
        return command.to_string();
    }
    if let Some(path) = env.get("PATH") {
        for dir in path.split(':') {
            let candidate = std::path::PathBuf::from(dir).join(command);
            if candidate.is_file() {
                return candidate.to_string_lossy().into_owned();
            }
        }
    }
    command.to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn shell_env_contains_path() {
        let env = get_shell_env();
        assert!(env.contains_key("PATH"), "shell env should contain PATH");
    }

    #[test]
    fn shell_env_is_cached() {
        let a = get_shell_env() as *const _;
        let b = get_shell_env() as *const _;
        assert_eq!(a, b);
    }

    #[test]
    fn parse_null_separated_basic() {
        let data = b"FOO=bar\0BAZ=qux\0";
        let env = parse_null_separated(data);
        assert_eq!(env.get("FOO").unwrap(), "bar");
        assert_eq!(env.get("BAZ").unwrap(), "qux");
    }

    #[test]
    fn parse_null_separated_value_with_newline() {
        let data = b"KEY=line1\nline2\0OTHER=val\0";
        let env = parse_null_separated(data);
        assert_eq!(env.get("KEY").unwrap(), "line1\nline2");
        assert_eq!(env.get("OTHER").unwrap(), "val");
    }

    #[test]
    fn parse_line_based_simple() {
        let data = b"FOO=bar\nBAZ=qux\n";
        let env = parse_line_based(data);
        assert_eq!(env.get("FOO").unwrap(), "bar");
        assert_eq!(env.get("BAZ").unwrap(), "qux");
    }

    #[test]
    fn parse_line_based_multiline_value() {
        let data = b"MULTI=line1\nline2\nNEXT=val\n";
        let env = parse_line_based(data);
        assert_eq!(env.get("MULTI").unwrap(), "line1\nline2");
        assert_eq!(env.get("NEXT").unwrap(), "val");
    }

    #[test]
    fn parse_line_based_value_with_equals() {
        let data = b"URL=https://example.com?a=1&b=2\n";
        let env = parse_line_based(data);
        assert_eq!(env.get("URL").unwrap(), "https://example.com?a=1&b=2");
    }

    #[test]
    fn resolve_command_absolute_unchanged() {
        let env = HashMap::new();
        assert_eq!(resolve_command("/usr/bin/node", &env), "/usr/bin/node");
    }

    #[test]
    fn resolve_command_not_found_returns_original() {
        let mut env = HashMap::new();
        env.insert("PATH".to_string(), "/nonexistent".to_string());
        assert_eq!(resolve_command("missing_bin", &env), "missing_bin");
    }

    #[test]
    fn is_valid_env_key_accepts_normal() {
        assert!(is_valid_env_key("PATH"));
        assert!(is_valid_env_key("HOME"));
        assert!(is_valid_env_key("MY_VAR_2"));
    }

    #[test]
    fn is_valid_env_key_rejects_invalid() {
        assert!(!is_valid_env_key(""));
        assert!(!is_valid_env_key("123"));
        assert!(!is_valid_env_key("has space"));
        assert!(!is_valid_env_key("has-dash"));
    }
}
