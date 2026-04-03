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

struct SkillSegment {
    skill_name: Option<String>,
    input: String,
}

static SKILL_LINE_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^/([a-z0-9-]+)(?:\s+(.*))?$").unwrap());

fn parse_input_for_skills(text: &str) -> Vec<SkillSegment> {
    let mut segments = Vec::new();
    let mut current_skill: Option<String> = None;
    let mut current_lines: Vec<&str> = Vec::new();

    for line in text.lines() {
        if let Some(caps) = SKILL_LINE_RE.captures(line) {
            if !current_lines.is_empty() || current_skill.is_some() {
                let input = current_lines.join("\n").trim().to_string();
                segments.push(SkillSegment {
                    skill_name: current_skill.take(),
                    input,
                });
                current_lines.clear();
            }
            current_skill = Some(caps[1].to_string());
            if let Some(rest) = caps.get(2) {
                current_lines.push(rest.as_str());
            }
        } else {
            current_lines.push(line);
        }
    }

    if !current_lines.is_empty() || current_skill.is_some() {
        let input = current_lines.join("\n").trim().to_string();
        segments.push(SkillSegment {
            skill_name: current_skill,
            input,
        });
    }

    segments.retain(|s| !s.input.is_empty() || s.skill_name.is_some());
    segments
}

static HAS_SKILL_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"(?m)^/[a-z0-9-]+(\s|$)").unwrap());

pub fn has_skill_references(text: &str) -> bool {
    HAS_SKILL_RE.is_match(text)
}

pub struct ResolveSkillResult {
    pub resolved_text: String,
    pub had_skills: bool,
}

pub fn resolve_skill_input(
    skill_service: &SkillService,
    text: &str,
) -> ResolveSkillResult {
    if !has_skill_references(text) {
        return ResolveSkillResult {
            resolved_text: text.to_string(),
            had_skills: false,
        };
    }

    let segments = parse_input_for_skills(text);
    let has_any_skill = segments.iter().any(|s| s.skill_name.is_some());
    if !has_any_skill {
        return ResolveSkillResult {
            resolved_text: text.to_string(),
            had_skills: false,
        };
    }

    let resolved: Vec<(Skill, String)> = segments
        .iter()
        .filter_map(|seg| {
            let skill_name = seg.skill_name.as_deref()?;
            let skill = skill_service.get_skill(skill_name)?.clone();
            Some((skill, seg.input.clone()))
        })
        .collect();

    if resolved.is_empty() {
        return ResolveSkillResult {
            resolved_text: text.to_string(),
            had_skills: false,
        };
    }

    let pairs: Vec<(&Skill, &str)> = resolved.iter().map(|(s, i)| (s, i.as_str())).collect();
    let composed = compose_multi_skill_user_message(&pairs);

    ResolveSkillResult {
        resolved_text: composed,
        had_skills: true,
    }
}

pub fn build_messages_from_tree(
    nodes: &[ConversationNodeForExecution],
    context_images: &[ImageData],
) -> Vec<ProcessedMessage> {
    let mut messages = Vec::new();
    let mut is_first_user = true;

    for node in nodes {
        if node.role == "user" {
            let has_context_images = is_first_user && !context_images.is_empty();
            let has_node_images = !node.images.is_empty();
            is_first_user = false;

            let mut text_content = node.content.clone();
            if !node.text_attachments.is_empty() {
                let wrapped: Vec<String> = node
                    .text_attachments
                    .iter()
                    .enumerate()
                    .map(|(i, t)| {
                        format!(
                            "<pasted-text name=\"Text #{}\">\n{}\n</pasted-text>",
                            i + 1,
                            t
                        )
                    })
                    .collect();
                let attachments_block = wrapped.join("\n\n");
                text_content = if text_content.is_empty() {
                    attachments_block
                } else {
                    format!("{attachments_block}\n\n{text_content}")
                };
            }

            if has_context_images || has_node_images {
                let mut parts = Vec::new();
                if has_context_images {
                    for (i, img) in context_images.iter().enumerate() {
                        parts.push(ContentPart::Text {
                            text: format!("[Context Image #{}]", i + 1),
                        });
                        parts.push(ContentPart::ImageUrl {
                            image_url: ImageUrlData {
                                url: format!("data:{};base64,{}", img.media_type, img.data),
                            },
                        });
                    }
                }
                for (i, img) in node.images.iter().enumerate() {
                    parts.push(ContentPart::Text {
                        text: format!("[Image #{}]", i + 1),
                    });
                    parts.push(ContentPart::ImageUrl {
                        image_url: ImageUrlData {
                            url: format!("data:{};base64,{}", img.media_type, img.data),
                        },
                    });
                }
                parts.push(ContentPart::Text {
                    text: text_content,
                });
                messages.push(ProcessedMessage {
                    role: "user".to_string(),
                    content: MessageContent::Parts(parts),
                });
            } else {
                messages.push(ProcessedMessage {
                    role: "user".to_string(),
                    content: MessageContent::Text(text_content),
                });
            }
        } else {
            messages.push(ProcessedMessage {
                role: "assistant".to_string(),
                content: MessageContent::Text(node.content.clone()),
            });
        }
    }

    messages
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
                assert_eq!(parts.len(), 3);
                assert!(matches!(&parts[0], ContentPart::Text { text } if text.contains("Context Image")));
                assert!(matches!(&parts[1], ContentPart::ImageUrl { .. }));
                assert!(matches!(&parts[2], ContentPart::Text { .. }));
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

    #[test]
    fn parse_single_skill() {
        let segments = parse_input_for_skills("/translate hello world");
        assert_eq!(segments.len(), 1);
        assert_eq!(segments[0].skill_name.as_deref(), Some("translate"));
        assert_eq!(segments[0].input, "hello world");
    }

    #[test]
    fn parse_multiple_skills() {
        let segments = parse_input_for_skills("/translate hello\n/formal world");
        assert_eq!(segments.len(), 2);
        assert_eq!(segments[0].skill_name.as_deref(), Some("translate"));
        assert_eq!(segments[0].input, "hello");
        assert_eq!(segments[1].skill_name.as_deref(), Some("formal"));
        assert_eq!(segments[1].input, "world");
    }

    #[test]
    fn parse_no_skills() {
        let segments = parse_input_for_skills("just plain text");
        assert_eq!(segments.len(), 1);
        assert!(segments[0].skill_name.is_none());
        assert_eq!(segments[0].input, "just plain text");
    }

    #[test]
    fn parse_skill_multiline_input() {
        let segments = parse_input_for_skills("/translate hello\nmore text\neven more");
        assert_eq!(segments.len(), 1);
        assert_eq!(segments[0].skill_name.as_deref(), Some("translate"));
        assert_eq!(segments[0].input, "hello\nmore text\neven more");
    }

    #[test]
    fn has_skill_references_detects_skills() {
        assert!(has_skill_references("/translate hello"));
        assert!(has_skill_references("some text\n/translate hello"));
        assert!(!has_skill_references("no skills here"));
        assert!(!has_skill_references("a /translate in middle"));
    }

    #[test]
    fn build_messages_simple_conversation() {
        let nodes = vec![
            ConversationNodeForExecution {
                node_id: "1".into(),
                role: "user".into(),
                content: "hello".into(),
                images: vec![],
                text_attachments: vec![],
            },
            ConversationNodeForExecution {
                node_id: "2".into(),
                role: "assistant".into(),
                content: "hi there".into(),
                images: vec![],
                text_attachments: vec![],
            },
        ];
        let messages = build_messages_from_tree(&nodes, &[]);
        assert_eq!(messages.len(), 2);
        assert_eq!(messages[0].role, "user");
        assert_eq!(messages[1].role, "assistant");
        if let MessageContent::Text(ref t) = messages[0].content {
            assert_eq!(t, "hello");
        } else {
            panic!("expected text");
        }
    }

    #[test]
    fn build_messages_with_text_attachments() {
        let nodes = vec![ConversationNodeForExecution {
            node_id: "1".into(),
            role: "user".into(),
            content: "analyze this".into(),
            images: vec![],
            text_attachments: vec!["some code".into()],
        }];
        let messages = build_messages_from_tree(&nodes, &[]);
        if let MessageContent::Text(ref t) = messages[0].content {
            assert!(t.contains("<pasted-text name=\"Text #1\">"));
            assert!(t.contains("some code"));
            assert!(t.contains("analyze this"));
        } else {
            panic!("expected text");
        }
    }

    #[test]
    fn build_messages_with_context_images() {
        let nodes = vec![ConversationNodeForExecution {
            node_id: "1".into(),
            role: "user".into(),
            content: "describe this".into(),
            images: vec![],
            text_attachments: vec![],
        }];
        let ctx_images = vec![ImageData {
            data: "abc123".into(),
            media_type: "image/png".into(),
        }];
        let messages = build_messages_from_tree(&nodes, &ctx_images);
        match &messages[0].content {
            MessageContent::Parts(parts) => {
                assert!(parts.len() >= 2);
                assert!(matches!(&parts[0], ContentPart::Text { text } if text.contains("Context Image")));
                assert!(matches!(&parts[1], ContentPart::ImageUrl { .. }));
            }
            _ => panic!("expected parts"),
        }
    }

    #[test]
    fn build_messages_context_images_only_on_first_user() {
        let nodes = vec![
            ConversationNodeForExecution {
                node_id: "1".into(),
                role: "user".into(),
                content: "first".into(),
                images: vec![],
                text_attachments: vec![],
            },
            ConversationNodeForExecution {
                node_id: "2".into(),
                role: "assistant".into(),
                content: "reply".into(),
                images: vec![],
                text_attachments: vec![],
            },
            ConversationNodeForExecution {
                node_id: "3".into(),
                role: "user".into(),
                content: "second".into(),
                images: vec![],
                text_attachments: vec![],
            },
        ];
        let ctx_images = vec![ImageData {
            data: "abc".into(),
            media_type: "image/png".into(),
        }];
        let messages = build_messages_from_tree(&nodes, &ctx_images);
        assert!(matches!(&messages[0].content, MessageContent::Parts(_)));
        assert!(matches!(&messages[2].content, MessageContent::Text(_)));
    }
}
