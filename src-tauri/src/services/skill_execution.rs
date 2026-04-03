use regex::Regex;
use std::sync::LazyLock;

use crate::models::context::ContextItem;
use crate::models::message::{
    ContentPart, ConversationNodeForExecution, ImageData, ImageUrlData, MessageContent,
    ProcessedMessage,
};
use crate::models::skill::Skill;
use crate::services::context::ContextManagerService;
use crate::services::skill::{SkillError, SkillService};

pub fn compose_skill_user_message(skill: &Skill, input: &str) -> String {
    format!(
        "<skill name=\"{}\">\n{}\n</skill>\n\n<input>\n{}\n</input>",
        skill.name, skill.body, input
    )
}

pub fn compose_multi_skill_user_message(segments: &[(&Skill, &str)]) -> String {
    let mut parts = Vec::new();

    for (i, (skill, input)) in segments.iter().enumerate() {
        parts.push(format!(
            "<skill name=\"{}\">\n{}\n</skill>\n\n<input>\n{}\n</input>",
            skill.name, skill.body, input
        ));

        if i < segments.len() - 1 {
            parts.push(format!("\n\n<skill-end name=\"{}\" />\n", skill.name));
        }
    }

    parts.join("")
}

pub fn prepare_skill_messages(
    system_prompt: &str,
    skill: &Skill,
    input: &str,
    context: &ContextManagerService,
) -> Vec<ProcessedMessage> {
    let mut messages = Vec::new();

    let system_content = if !context.get_context_or_default("").is_empty() {
        let ctx = context.get_context_or_default("");
        format!("{system_prompt}\n\n<context>\n{ctx}\n</context>")
    } else {
        system_prompt.to_string()
    };

    messages.push(ProcessedMessage {
        role: "system".to_string(),
        content: MessageContent::Text(system_content),
    });

    let user_text = compose_skill_user_message(skill, input);

    if context.has_images() {
        let mut parts = Vec::new();
        let mut img_index = 1;
        for item in &context.get_items() {
            if let ContextItem::Image { data, media_type } = item {
                parts.push(ContentPart::Text {
                    text: format!("[Context Image #{img_index}]"),
                });
                img_index += 1;
                parts.push(ContentPart::ImageUrl {
                    image_url: ImageUrlData {
                        url: format!("data:{media_type};base64,{data}"),
                    },
                });
            }
        }
        parts.push(ContentPart::Text { text: user_text });
        messages.push(ProcessedMessage {
            role: "user".to_string(),
            content: MessageContent::Parts(parts),
        });
    } else {
        messages.push(ProcessedMessage {
            role: "user".to_string(),
            content: MessageContent::Text(user_text),
        });
    }

    messages
}

pub fn resolve_skill_or_err(
    skill_service: &crate::services::skill::SkillService,
    skill_name: &str,
) -> Result<Skill, SkillError> {
    skill_service
        .get_skill(skill_name)
        .cloned()
        .ok_or_else(|| SkillError::NotFound(skill_name.to_string()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn test_skill(name: &str, body: &str) -> Skill {
        Skill {
            name: name.to_string(),
            display_name: name.to_string(),
            description: None,
            body: body.to_string(),
            file_path: PathBuf::new(),
        }
    }

    #[test]
    fn compose_single_skill_message() {
        let skill = test_skill("translate-english", "Translate to English.");
        let result = compose_skill_user_message(&skill, "Cześć!");
        assert!(result.contains("<skill name=\"translate-english\">"));
        assert!(result.contains("Translate to English."));
        assert!(result.contains("</skill>"));
        assert!(result.contains("<input>"));
        assert!(result.contains("Cześć!"));
        assert!(result.contains("</input>"));
    }

    #[test]
    fn compose_multi_skill_message() {
        let skill1 = test_skill("translate", "Translate.");
        let skill2 = test_skill("formal", "Make formal.");

        let segments: Vec<(&Skill, &str)> = vec![(&skill1, "hello"), (&skill2, "world")];
        let result = compose_multi_skill_user_message(&segments);

        assert!(result.contains("<skill name=\"translate\">"));
        assert!(result.contains("<skill-end name=\"translate\" />"));
        assert!(result.contains("<skill name=\"formal\">"));
        assert!(result.contains("hello"));
        assert!(result.contains("world"));
        assert!(!result.contains("<skill-end name=\"formal\" />"));
    }

    #[test]
    fn prepare_messages_without_context() {
        let skill = test_skill("test", "Test body.");
        let context = ContextManagerService::new();
        let messages = prepare_skill_messages("You are helpful.", &skill, "input text", &context);

        assert_eq!(messages.len(), 2);
        assert_eq!(messages[0].role, "system");
        assert_eq!(messages[1].role, "user");

        if let MessageContent::Text(ref text) = messages[0].content {
            assert_eq!(text, "You are helpful.");
        } else {
            panic!("expected text content for system message");
        }

        if let MessageContent::Text(ref text) = messages[1].content {
            assert!(text.contains("<skill name=\"test\">"));
            assert!(text.contains("input text"));
        } else {
            panic!("expected text content for user message");
        }
    }

    #[test]
    fn prepare_messages_with_text_context() {
        let skill = test_skill("test", "Test body.");
        let mut context = ContextManagerService::new();
        context.set_context("background info".into());
        let messages = prepare_skill_messages("You are helpful.", &skill, "input", &context);

        if let MessageContent::Text(ref text) = messages[0].content {
            assert!(text.contains("You are helpful."));
            assert!(text.contains("<context>"));
            assert!(text.contains("background info"));
        } else {
            panic!("expected text content for system message");
        }
    }

    #[test]
    fn prepare_messages_with_images() {
        let skill = test_skill("test", "Test body.");
        let mut context = ContextManagerService::new();
        context.append_context_image("abc123".into(), "image/png".into());
        let messages = prepare_skill_messages("System.", &skill, "input", &context);

        assert_eq!(messages.len(), 2);
        match &messages[1].content {
            MessageContent::Parts(parts) => {
                assert_eq!(parts.len(), 2);
                assert!(matches!(&parts[0], ContentPart::Text { .. }));
                assert!(matches!(&parts[1], ContentPart::ImageUrl { .. }));
            }
            _ => panic!("expected parts content for user message with images"),
        }
    }

    #[test]
    fn single_segment_no_skill_end() {
        let skill = test_skill("only", "Body.");
        let segments: Vec<(&Skill, &str)> = vec![(&skill, "text")];
        let result = compose_multi_skill_user_message(&segments);
        assert!(!result.contains("<skill-end"));
    }
}
