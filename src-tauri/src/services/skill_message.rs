use regex::Regex;
use std::collections::HashMap;
use std::sync::LazyLock;

use crate::models::context::ContextItem;
use crate::models::message::{
    AppliedSkill, ContentPart, ConversationNodeForExecution, ImageData, ImageUrlData,
    MessageContent, NodeUpdate, ProcessedMessage,
};
use crate::models::skill::Skill;
use crate::services::context::ContextManagerService;
use crate::services::execution::substitute_environment_placeholders;
use crate::services::skill::{SkillError, SkillService};

pub fn compose_skill_user_message(
    skill: &Skill,
    input: &str,
    active_app: &str,
    recent_apps: &str,
) -> String {
    let resolved_body =
        substitute_environment_placeholders(&skill.body, active_app, recent_apps);
    format!(
        "<skill name=\"{}\">\n{}\n</skill>\n\n<input>\n{}\n</input>",
        skill.name, resolved_body, input
    )
}

pub fn compose_from_snapshot(
    skills: &[AppliedSkill],
    bodies: &HashMap<i64, String>,
) -> String {
    let mut parts = Vec::new();

    for (i, applied) in skills.iter().enumerate() {
        let empty = String::new();
        let body = bodies.get(&applied.skill_version_id).unwrap_or(&empty);
        parts.push(format!(
            "<skill name=\"{}\">\n{}\n</skill>\n\n<input>\n{}\n</input>",
            applied.name, body, applied.input
        ));

        if i < skills.len() - 1 {
            parts.push(format!("\n\n<skill-end name=\"{}\" />\n", applied.name));
        }
    }

    parts.join("")
}

pub fn collect_skill_version_ids(nodes: &[ConversationNodeForExecution]) -> Vec<i64> {
    let mut ids = Vec::new();
    for node in nodes {
        for applied in &node.applied_skills {
            ids.push(applied.skill_version_id);
        }
    }
    ids.sort_unstable();
    ids.dedup();
    ids
}

pub fn load_skill_version_bodies(
    conn: &rusqlite::Connection,
    ids: &[i64],
) -> Result<HashMap<i64, String>, rusqlite::Error> {
    let mut bodies = HashMap::new();
    if ids.is_empty() {
        return Ok(bodies);
    }
    let placeholders: Vec<String> = (1..=ids.len()).map(|i| format!("?{i}")).collect();
    let sql = format!(
        "SELECT id, body FROM skill_versions WHERE id IN ({})",
        placeholders.join(",")
    );
    let mut stmt = conn.prepare(&sql)?;
    let params: Vec<&dyn rusqlite::ToSql> =
        ids.iter().map(|i| i as &dyn rusqlite::ToSql).collect();
    let rows = stmt.query_map(params.as_slice(), |row| {
        Ok((row.get::<_, i64>(0)?, row.get::<_, String>(1)?))
    })?;
    for row in rows {
        let (id, body) = row?;
        bodies.insert(id, body);
    }
    Ok(bodies)
}

pub fn prepare_skill_messages(
    system_prompt: &str,
    skill: &Skill,
    input: &str,
    context: &ContextManagerService,
    active_app: &str,
    recent_apps: &str,
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
        tool_calls: None,
        tool_call_id: None,
    });

    let user_text = compose_skill_user_message(skill, input, active_app, recent_apps);

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
            tool_calls: None,
            tool_call_id: None,
        });
    } else {
        messages.push(ProcessedMessage {
            role: "user".to_string(),
            content: MessageContent::Text(user_text),
            tool_calls: None,
            tool_call_id: None,
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

static SKILL_TOKEN_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"(?m)(?:^|\s)(/[a-z0-9-]+)").unwrap());

fn parse_input_for_skills(
    text: &str,
    is_registered: impl Fn(&str) -> bool,
) -> Vec<SkillSegment> {
    let skill_positions: Vec<(usize, usize, String)> = SKILL_TOKEN_RE
        .captures_iter(text)
        .filter_map(|caps| {
            let m = caps.get(1).unwrap();
            let name = m.as_str()[1..].to_string();
            is_registered(&name).then_some((m.start(), m.end(), name))
        })
        .collect();

    if skill_positions.is_empty() {
        let trimmed = text.trim().to_string();
        if trimmed.is_empty() {
            return Vec::new();
        }
        return vec![SkillSegment {
            skill_name: None,
            input: trimmed,
        }];
    }

    let mut segments = Vec::new();

    let before = text[..skill_positions[0].0].trim();
    if !before.is_empty() {
        segments.push(SkillSegment {
            skill_name: None,
            input: before.to_string(),
        });
    }

    for (i, (_, end, name)) in skill_positions.iter().enumerate() {
        let input_end = skill_positions
            .get(i + 1)
            .map(|(start, _, _)| *start)
            .unwrap_or(text.len());
        let input = text[*end..input_end].trim().to_string();
        segments.push(SkillSegment {
            skill_name: Some(name.clone()),
            input,
        });
    }

    segments.retain(|s| !s.input.is_empty() || s.skill_name.is_some());
    segments
}

pub fn has_skill_references(text: &str) -> bool {
    SKILL_TOKEN_RE.is_match(text)
}

pub struct ResolveSkillResult {
    pub had_skills: bool,
    pub applied_skills: Vec<AppliedSkill>,
}

pub fn resolve_skill_input(
    skill_service: &SkillService,
    text: &str,
) -> ResolveSkillResult {
    if !has_skill_references(text) {
        return ResolveSkillResult {
            had_skills: false,
            applied_skills: vec![],
        };
    }

    let segments = parse_input_for_skills(text, |name| skill_service.get_skill(name).is_some());
    let applied_skills: Vec<AppliedSkill> = segments
        .iter()
        .filter_map(|seg| {
            let skill_name = seg.skill_name.as_deref()?;
            let skill = skill_service.get_skill(skill_name)?;
            let skill_version_id = skill.skill_version_id?;
            Some(AppliedSkill {
                name: skill.name.clone(),
                skill_version_id,
                input: seg.input.clone(),
            })
        })
        .collect();

    if applied_skills.is_empty() {
        return ResolveSkillResult {
            had_skills: false,
            applied_skills: vec![],
        };
    }

    ResolveSkillResult {
        had_skills: true,
        applied_skills,
    }
}

fn build_user_text_with_updates(content: &str, updates: &[NodeUpdate]) -> String {
    let mut prefix_parts: Vec<String> = Vec::new();
    for update in updates {
        match update {
            NodeUpdate::Environment { value } => {
                prefix_parts.push(value.clone());
            }
            NodeUpdate::Context {
                content,
                reason,
                ..
            } => match reason.as_str() {
                "initial" => {
                    prefix_parts.push(format!("<context>\n{content}\n</context>"));
                }
                "replaced" => {
                    prefix_parts.push(format!("<context-update>\n{content}\n</context-update>"));
                }
                "cleared" => {
                    prefix_parts.push("<context-update reason=\"cleared\" />".to_string());
                }
                _ => {}
            },
        }
    }
    if prefix_parts.is_empty() {
        content.to_string()
    } else {
        let prefix = prefix_parts.join("\n\n");
        format!("{prefix}\n\n{content}")
    }
}

pub fn build_messages_from_tree(
    nodes: &[ConversationNodeForExecution],
    context_images: &[ImageData],
    skill_bodies: &HashMap<i64, String>,
) -> Vec<ProcessedMessage> {
    let mut messages = Vec::new();
    let mut is_first_user = true;

    for node in nodes {
        if node.role == "user" {
            let has_context_images = is_first_user && !context_images.is_empty();
            let has_node_images = !node.images.is_empty();
            is_first_user = false;

            let composed_content = if !node.applied_skills.is_empty() {
                compose_from_snapshot(&node.applied_skills, skill_bodies)
            } else {
                node.content.clone()
            };

            let mut text_content = build_user_text_with_updates(&composed_content, &node.updates);
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
                    tool_calls: None,
                    tool_call_id: None,
                });
            } else {
                messages.push(ProcessedMessage {
                    role: "user".to_string(),
                    content: MessageContent::Text(text_content),
                    tool_calls: None,
                    tool_call_id: None,
                });
            }
        } else {
            messages.push(ProcessedMessage {
                role: "assistant".to_string(),
                content: MessageContent::Text(node.content.clone()),
                tool_calls: None,
                tool_call_id: None,
            });
        }
    }

    if let Some(last) = messages.last() {
        if last.role == "assistant" {
            if let MessageContent::Text(ref text) = last.content {
                if text.is_empty() {
                    messages.pop();
                }
            }
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
            model: None,
            parameters: None,
            body: body.to_string(),
            file_path: PathBuf::new(),
            skill_version_id: Some(1),
        }
    }

    fn snapshot_bodies(items: &[(i64, &str)]) -> HashMap<i64, String> {
        items.iter().map(|(id, body)| (*id, (*body).to_string())).collect()
    }

    #[test]
    fn compose_single_skill_message() {
        let skill = test_skill("translate-english", "Translate to English.");
        let result = compose_skill_user_message(&skill, "Cześć!", "", "");
        assert!(result.contains("<skill name=\"translate-english\">"));
        assert!(result.contains("Translate to English."));
        assert!(result.contains("</skill>"));
        assert!(result.contains("<input>"));
        assert!(result.contains("Cześć!"));
        assert!(result.contains("</input>"));
    }

    #[test]
    fn compose_multi_skill_from_snapshot() {
        let skills = vec![
            AppliedSkill {
                name: "translate".into(),
                skill_version_id: 1,
                input: "hello".into(),
            },
            AppliedSkill {
                name: "formal".into(),
                skill_version_id: 2,
                input: "world".into(),
            },
        ];
        let bodies = snapshot_bodies(&[(1, "Translate."), (2, "Make formal.")]);
        let result = compose_from_snapshot(&skills, &bodies);

        assert!(result.contains("<skill name=\"translate\">"));
        assert!(result.contains("<skill-end name=\"translate\" />"));
        assert!(result.contains("<skill name=\"formal\">"));
        assert!(result.contains("hello"));
        assert!(result.contains("world"));
        assert!(!result.contains("<skill-end name=\"formal\" />"));
    }

    #[test]
    fn compose_from_snapshot_single_no_skill_end() {
        let skills = vec![AppliedSkill {
            name: "only".into(),
            skill_version_id: 1,
            input: "text".into(),
        }];
        let bodies = snapshot_bodies(&[(1, "Body.")]);
        let result = compose_from_snapshot(&skills, &bodies);
        assert!(!result.contains("<skill-end"));
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
    fn parse_single_skill() {
        let segments = parse_input_for_skills("/translate hello world", |_| true);
        assert_eq!(segments.len(), 1);
        assert_eq!(segments[0].skill_name.as_deref(), Some("translate"));
        assert_eq!(segments[0].input, "hello world");
    }

    #[test]
    fn parse_multiple_skills() {
        let segments = parse_input_for_skills("/translate hello\n/formal world", |_| true);
        assert_eq!(segments.len(), 2);
        assert_eq!(segments[0].skill_name.as_deref(), Some("translate"));
        assert_eq!(segments[0].input, "hello");
        assert_eq!(segments[1].skill_name.as_deref(), Some("formal"));
        assert_eq!(segments[1].input, "world");
    }

    #[test]
    fn parse_no_skills() {
        let segments = parse_input_for_skills("just plain text", |_| true);
        assert_eq!(segments.len(), 1);
        assert!(segments[0].skill_name.is_none());
        assert_eq!(segments[0].input, "just plain text");
    }

    #[test]
    fn parse_skill_multiline_input() {
        let segments =
            parse_input_for_skills("/translate hello\nmore text\neven more", |_| true);
        assert_eq!(segments.len(), 1);
        assert_eq!(segments[0].skill_name.as_deref(), Some("translate"));
        assert_eq!(segments[0].input, "hello\nmore text\neven more");
    }

    #[test]
    fn parse_keeps_unregistered_skill_as_text() {
        let segments = parse_input_for_skills(
            "/todo dodaj /plan-task coś tam",
            |name| name == "todo",
        );
        assert_eq!(segments.len(), 1);
        assert_eq!(segments[0].skill_name.as_deref(), Some("todo"));
        assert_eq!(segments[0].input, "dodaj /plan-task coś tam");
    }

    #[test]
    fn parse_all_unregistered_returns_plain_text() {
        let segments = parse_input_for_skills("/plan-task coś tam", |_| false);
        assert_eq!(segments.len(), 1);
        assert!(segments[0].skill_name.is_none());
        assert_eq!(segments[0].input, "/plan-task coś tam");
    }

    #[test]
    fn has_skill_references_detects_skills() {
        assert!(has_skill_references("/translate hello"));
        assert!(has_skill_references("some text\n/translate hello"));
        assert!(!has_skill_references("no skills here"));
        assert!(has_skill_references("a /translate in middle"));
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
                updates: vec![],
                applied_skills: vec![],
            },
            ConversationNodeForExecution {
                node_id: "2".into(),
                role: "assistant".into(),
                content: "hi there".into(),
                images: vec![],
                text_attachments: vec![],
                updates: vec![],
                applied_skills: vec![],
            },
        ];
        let messages = build_messages_from_tree(&nodes, &[], &HashMap::new());
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
            updates: vec![],
            applied_skills: vec![],
        }];
        let messages = build_messages_from_tree(&nodes, &[], &HashMap::new());
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
            updates: vec![],
            applied_skills: vec![],
        }];
        let ctx_images = vec![ImageData {
            data: "abc123".into(),
            media_type: "image/png".into(),
        }];
        let messages = build_messages_from_tree(&nodes, &ctx_images, &HashMap::new());
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
                updates: vec![],
                applied_skills: vec![],
            },
            ConversationNodeForExecution {
                node_id: "2".into(),
                role: "assistant".into(),
                content: "reply".into(),
                images: vec![],
                text_attachments: vec![],
                updates: vec![],
                applied_skills: vec![],
            },
            ConversationNodeForExecution {
                node_id: "3".into(),
                role: "user".into(),
                content: "second".into(),
                images: vec![],
                text_attachments: vec![],
                updates: vec![],
                applied_skills: vec![],
            },
        ];
        let ctx_images = vec![ImageData {
            data: "abc".into(),
            media_type: "image/png".into(),
        }];
        let messages = build_messages_from_tree(&nodes, &ctx_images, &HashMap::new());
        assert!(matches!(&messages[0].content, MessageContent::Parts(_)));
        assert!(matches!(&messages[2].content, MessageContent::Text(_)));
    }
}
