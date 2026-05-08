use std::sync::LazyLock;

use regex::Regex;
use serde::Serialize;

#[derive(Default, Clone, Debug)]
pub struct PlaceholderContext {
    pub active_app: String,
    pub recent_apps: String,
}

impl PlaceholderContext {
    pub fn empty() -> Self {
        Self::default()
    }

    pub fn with_apps(active_app: impl Into<String>, recent_apps: impl Into<String>) -> Self {
        Self {
            active_app: active_app.into(),
            recent_apps: recent_apps.into(),
        }
    }
}

pub struct Placeholder {
    pub name: &'static str,
    pub label: &'static str,
    pub description: &'static str,
    resolve: fn(&PlaceholderContext) -> String,
    preview: fn(&PlaceholderContext) -> String,
}

impl Placeholder {
    pub fn token(&self) -> String {
        format!("{{{{{}}}}}", self.name)
    }

    pub fn resolve(&self, ctx: &PlaceholderContext) -> String {
        (self.resolve)(ctx)
    }

    pub fn preview(&self, ctx: &PlaceholderContext) -> String {
        (self.preview)(ctx)
    }
}

fn resolve_date(_: &PlaceholderContext) -> String {
    chrono::Local::now().format("%Y-%m-%d").to_string()
}
fn resolve_time(_: &PlaceholderContext) -> String {
    chrono::Local::now().format("%H:%M").to_string()
}
fn resolve_timezone(_: &PlaceholderContext) -> String {
    chrono::Local::now().format("%Z").to_string()
}
fn resolve_os(_: &PlaceholderContext) -> String {
    std::env::consts::OS.to_string()
}
fn resolve_active_app(ctx: &PlaceholderContext) -> String {
    ctx.active_app.clone()
}
fn resolve_recent_apps(ctx: &PlaceholderContext) -> String {
    ctx.recent_apps.clone()
}

pub static REGISTRY: &[Placeholder] = &[
    Placeholder {
        name: "date",
        label: "Date",
        description: "Local date in YYYY-MM-DD format.",
        resolve: resolve_date,
        preview: resolve_date,
    },
    Placeholder {
        name: "time",
        label: "Time",
        description: "Local time in HH:MM (24h) format.",
        resolve: resolve_time,
        preview: resolve_time,
    },
    Placeholder {
        name: "timezone",
        label: "Timezone",
        description: "System timezone abbreviation.",
        resolve: resolve_timezone,
        preview: resolve_timezone,
    },
    Placeholder {
        name: "os",
        label: "OS",
        description: "Operating system identifier (linux, macos, windows).",
        resolve: resolve_os,
        preview: resolve_os,
    },
    Placeholder {
        name: "active_app",
        label: "Active app",
        description: "Foreground application at the moment the conversation started.",
        resolve: resolve_active_app,
        preview: resolve_active_app,
    },
    Placeholder {
        name: "recent_apps",
        label: "Recent apps",
        description: "Comma-separated list of recently focused applications.",
        resolve: resolve_recent_apps,
        preview: resolve_recent_apps,
    },
];

static TOKEN_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"\{\{([a-z_][a-z0-9_]*)\}\}").unwrap());

pub fn substitute(text: &str, ctx: &PlaceholderContext) -> String {
    if text.is_empty() {
        return String::new();
    }
    TOKEN_RE
        .replace_all(text, |caps: &regex::Captures| {
            let name = &caps[1];
            match REGISTRY.iter().find(|p| p.name == name) {
                Some(p) => p.resolve(ctx),
                None => caps[0].to_string(),
            }
        })
        .into_owned()
}

#[derive(Serialize, Clone)]
pub struct PlaceholderInfo {
    pub token: String,
    pub name: String,
    pub label: String,
    pub description: String,
    pub example: String,
}

pub fn list(ctx: &PlaceholderContext) -> Vec<PlaceholderInfo> {
    REGISTRY
        .iter()
        .map(|p| PlaceholderInfo {
            token: p.token(),
            name: p.name.to_string(),
            label: p.label.to_string(),
            description: p.description.to_string(),
            example: p.preview(ctx),
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn substitute_replaces_known_tokens() {
        let ctx = PlaceholderContext::with_apps("Firefox", "Firefox, Code");
        let result = substitute("os={{os}} app={{active_app}}", &ctx);
        assert_eq!(result, format!("os={} app=Firefox", std::env::consts::OS));
    }

    #[test]
    fn substitute_leaves_unknown_tokens_literal() {
        let ctx = PlaceholderContext::empty();
        let result = substitute("hello {{unknown_token}} world", &ctx);
        assert_eq!(result, "hello {{unknown_token}} world");
    }

    #[test]
    fn substitute_handles_empty_string() {
        let ctx = PlaceholderContext::empty();
        assert_eq!(substitute("", &ctx), "");
    }

    #[test]
    fn substitute_handles_text_without_tokens() {
        let ctx = PlaceholderContext::empty();
        assert_eq!(substitute("plain text", &ctx), "plain text");
    }

    #[test]
    fn substitute_resolves_recent_apps() {
        let ctx = PlaceholderContext::with_apps("Firefox", "Firefox, VS Code, Slack");
        let result = substitute("recent: {{recent_apps}}", &ctx);
        assert_eq!(result, "recent: Firefox, VS Code, Slack");
    }

    #[test]
    fn list_includes_all_registered_tokens() {
        let infos = list(&PlaceholderContext::empty());
        let names: Vec<_> = infos.iter().map(|i| i.name.as_str()).collect();
        assert!(names.contains(&"date"));
        assert!(names.contains(&"time"));
        assert!(names.contains(&"timezone"));
        assert!(names.contains(&"os"));
        assert!(names.contains(&"active_app"));
        assert!(names.contains(&"recent_apps"));
        assert_eq!(infos.len(), REGISTRY.len());
    }

    #[test]
    fn list_renders_token_with_braces() {
        let infos = list(&PlaceholderContext::empty());
        let date = infos.iter().find(|i| i.name == "date").unwrap();
        assert_eq!(date.token, "{{date}}");
    }

    #[test]
    fn list_uses_real_active_app_for_preview() {
        let ctx = PlaceholderContext::with_apps("Cursor", "Cursor, Brave");
        let infos = list(&ctx);
        let active = infos.iter().find(|i| i.name == "active_app").unwrap();
        assert_eq!(active.example, "Cursor");
    }
}
