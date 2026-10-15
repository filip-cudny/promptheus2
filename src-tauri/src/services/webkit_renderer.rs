use std::path::Path;

const FORCE_VARS: [(&str, &str); 2] = [
    ("WEBKIT_FORCE_DMABUF_RENDERER", "1"),
    ("WEBKIT_DMABUF_RENDERER_FORCE_SHM", "1"),
];

const USER_CONTROL_VARS: [&str; 4] = [
    "WEBKIT_FORCE_DMABUF_RENDERER",
    "WEBKIT_DMABUF_RENDERER_FORCE_SHM",
    "WEBKIT_DISABLE_DMABUF_RENDERER",
    "WEBKIT_DISABLE_COMPOSITING_MODE",
];

#[derive(Debug, PartialEq)]
enum Decision {
    Set(&'static [(&'static str, &'static str)]),
    Skip(String),
}

fn decide(nvidia_loaded: bool, present_vars: &[&str]) -> Decision {
    if let Some(name) = present_vars.first() {
        return Decision::Skip(format!("{name} set by user"));
    }
    if !nvidia_loaded {
        return Decision::Skip("no NVIDIA driver".to_string());
    }
    Decision::Set(&FORCE_VARS)
}

/// Must run before any thread starts; returns the log line to emit once the log plugin is up.
pub fn configure() -> String {
    let present: Vec<&str> = USER_CONTROL_VARS
        .into_iter()
        .filter(|name| std::env::var_os(name).is_some())
        .collect();
    match decide(Path::new("/sys/module/nvidia").exists(), &present) {
        Decision::Set(vars) => {
            let assignments: Vec<String> = vars
                .iter()
                .map(|(name, value)| {
                    std::env::set_var(name, value);
                    format!("{name}={value}")
                })
                .collect();
            format!(
                "WebKit renderer: set {} (NVIDIA driver loaded)",
                assignments.join(" ")
            )
        }
        Decision::Skip(reason) => format!("WebKit renderer: unchanged ({reason})"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn nvidia_without_user_vars_sets_both_to_one() {
        assert_eq!(
            decide(true, &[]),
            Decision::Set(&[
                ("WEBKIT_FORCE_DMABUF_RENDERER", "1"),
                ("WEBKIT_DMABUF_RENDERER_FORCE_SHM", "1"),
            ])
        );
    }

    #[test]
    fn no_nvidia_skips() {
        assert_eq!(
            decide(false, &[]),
            Decision::Skip("no NVIDIA driver".to_string())
        );
    }

    #[test]
    fn each_user_var_skips_and_is_named() {
        for name in USER_CONTROL_VARS {
            match decide(true, &[name]) {
                Decision::Skip(reason) => assert!(reason.contains(name), "{reason}"),
                other => panic!("{name}: {other:?}"),
            }
        }
    }
}
