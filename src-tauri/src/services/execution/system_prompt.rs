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
    let about_you = config.about_you();
    let preferred_name = config.preferred_name();

    let environment_section = resolved_environment_section
        .map(|s| s.to_string())
        .unwrap_or_else(|| resolve_environment_section_template(config, active_app, recent_apps));

    assemble_system_prompt(
        &system_prompt,
        preferred_name,
        &about_you,
        &environment_section,
        &input_format_guide,
    )
}

fn assemble_system_prompt(
    system_prompt: &str,
    preferred_name: &str,
    about_you: &str,
    environment_section: &str,
    input_format_guide: &str,
) -> String {
    let mut sections: Vec<String> = Vec::new();

    let system_trimmed = system_prompt.trim();
    if !system_trimmed.is_empty() {
        sections.push(system_trimmed.to_string());
    }

    let user_context = build_user_context_section(preferred_name, about_you);
    if let Some(section) = user_context {
        sections.push(section);
    }

    let env_trimmed = environment_section.trim();
    if !env_trimmed.is_empty() {
        sections.push(format!("<environment>\n{env_trimmed}\n</environment>"));
    }

    let input_format_trimmed = input_format_guide.trim();
    if !input_format_trimmed.is_empty() {
        sections.push(format!(
            "<input_format>\n{input_format_trimmed}\n</input_format>"
        ));
    }

    sections.join("\n\n")
}

fn build_user_context_section(preferred_name: &str, about_you: &str) -> Option<String> {
    let name_trimmed = preferred_name.trim();
    let about_trimmed = about_you.trim();
    if name_trimmed.is_empty() && about_trimmed.is_empty() {
        return None;
    }

    let mut body = String::new();
    if !name_trimmed.is_empty() {
        body.push_str(&format!("Name: {name_trimmed}"));
        if !about_trimmed.is_empty() {
            body.push_str("\n\n");
        }
    }
    if !about_trimmed.is_empty() {
        body.push_str(about_trimmed);
    }

    Some(format!("<user_context>\n{body}\n</user_context>"))
}

#[cfg(test)]
mod tests {
    use super::{assemble_system_prompt, build_user_context_section};

    const SYSTEM: &str = "You are a candid, direct assistant.";
    const ABOUT_YOU: &str = "I am a software engineer.";
    const ENV: &str = "- **Date**: 2026-05-08";
    const INPUT_FORMAT: &str = "Interpret structured tags.";

    #[test]
    fn full_scaffold_contains_all_sections_in_order() {
        let result = assemble_system_prompt(SYSTEM, "Filip", ABOUT_YOU, ENV, INPUT_FORMAT);
        let expected = "You are a candid, direct assistant.\n\n\
<user_context>\nName: Filip\n\nI am a software engineer.\n</user_context>\n\n\
<environment>\n- **Date**: 2026-05-08\n</environment>\n\n\
<input_format>\nInterpret structured tags.\n</input_format>";
        assert_eq!(result, expected);
    }

    #[test]
    fn empty_environment_omits_environment_section() {
        let result = assemble_system_prompt(SYSTEM, "Filip", ABOUT_YOU, "", INPUT_FORMAT);
        assert!(!result.contains("<environment>"));
        assert!(result.contains("<user_context>"));
        assert!(result.contains("<input_format>"));
    }

    #[test]
    fn empty_preferred_name_omits_name_line_keeps_user_context() {
        let result = assemble_system_prompt(SYSTEM, "", ABOUT_YOU, ENV, INPUT_FORMAT);
        assert!(result.contains("<user_context>"));
        assert!(!result.contains("Name:"));
        assert!(result.contains("I am a software engineer."));
    }

    #[test]
    fn empty_preferred_name_and_about_you_omits_user_context_section() {
        let result = assemble_system_prompt(SYSTEM, "", "", ENV, INPUT_FORMAT);
        assert!(!result.contains("<user_context>"));
        assert!(result.contains("<environment>"));
        assert!(result.contains("<input_format>"));
    }

    #[test]
    fn whitespace_only_preferred_name_treated_as_empty() {
        let section = build_user_context_section("   \t", ABOUT_YOU).unwrap();
        assert!(!section.contains("Name:"));
        assert!(section.contains("I am a software engineer."));
    }

    #[test]
    fn separator_dashes_no_longer_present() {
        let result = assemble_system_prompt(SYSTEM, "Filip", ABOUT_YOU, ENV, INPUT_FORMAT);
        assert!(!result.contains("\n\n---\n\n"));
    }

    #[test]
    fn user_context_with_only_name() {
        let section = build_user_context_section("Filip", "").unwrap();
        assert_eq!(section, "<user_context>\nName: Filip\n</user_context>");
    }

    #[test]
    fn user_context_with_only_about_you() {
        let section = build_user_context_section("", "Backend engineer.").unwrap();
        assert_eq!(
            section,
            "<user_context>\nBackend engineer.\n</user_context>"
        );
    }

    #[test]
    fn empty_system_prompt_dropped() {
        let result = assemble_system_prompt("   ", "Filip", ABOUT_YOU, ENV, INPUT_FORMAT);
        assert!(result.starts_with("<user_context>"));
    }
}
