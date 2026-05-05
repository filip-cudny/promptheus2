use crate::services::config::ConfigService;

pub fn resolve_environment_section_template(
    config: &ConfigService,
    active_app: &str,
    recent_apps: &str,
) -> String {
    let template = config.environment_section_template();
    if template.is_empty() {
        return String::new();
    }

    let now = chrono::Local::now();
    template
        .replace("{{date}}", &now.format("%Y-%m-%d").to_string())
        .replace("{{time}}", &now.format("%H:%M").to_string())
        .replace("{{timezone}}", &now.format("%Z").to_string())
        .replace("{{os}}", std::env::consts::OS)
        .replace("{{active_app}}", active_app)
        .replace("{{recent_apps}}", recent_apps)
}

pub fn build_system_prompt_base(
    config: &ConfigService,
    resolved_environment_section: Option<&str>,
    active_app: &str,
    recent_apps: &str,
) -> String {
    let system_prompt = config.system_prompt();
    let input_format_guide = config.input_format_guide();
    let about_me = config.about_me();

    let environment_section = resolved_environment_section
        .map(|s| s.to_string())
        .unwrap_or_else(|| resolve_environment_section_template(config, active_app, recent_apps));

    if environment_section.is_empty() {
        format!("{system_prompt}\n\n---\n\n{input_format_guide}\n\n---\n\n{about_me}")
    } else {
        format!(
            "{system_prompt}\n\n---\n\n{environment_section}\n\n---\n\n{input_format_guide}\n\n---\n\n{about_me}"
        )
    }
}
