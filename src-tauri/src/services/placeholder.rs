use std::collections::HashMap;

use log::warn;
use regex::Regex;

use crate::models::context::ContextItem;
use crate::models::message::{ContentPart, ImageUrlData, MessageContent, ProcessedMessage};
use crate::services::clipboard::{ClipboardError, ClipboardService};
use crate::services::context::ContextManagerService;

#[derive(Debug, thiserror::Error)]
pub enum PlaceholderError {
    #[error("clipboard unavailable: {0}")]
    ClipboardUnavailable(String),

    #[error("placeholder processing failed for '{0}': {1}")]
    ProcessorFailed(String, String),
}

pub trait PlaceholderProcessor {
    fn name(&self) -> &str;
    fn description(&self) -> &str;
    fn process(
        &self,
        context_override: Option<&str>,
        clipboard: &ClipboardService,
        context_mgr: &ContextManagerService,
    ) -> Result<String, PlaceholderError>;
}

struct ClipboardProcessor;

impl PlaceholderProcessor for ClipboardProcessor {
    fn name(&self) -> &str {
        "clipboard"
    }

    fn description(&self) -> &str {
        "The current clipboard text content"
    }

    fn process(
        &self,
        context_override: Option<&str>,
        clipboard: &ClipboardService,
        _context_mgr: &ContextManagerService,
    ) -> Result<String, PlaceholderError> {
        if let Some(s) = context_override {
            if s.trim().is_empty() {
                return Err(PlaceholderError::ClipboardUnavailable(
                    "Clipboard is empty".into(),
                ));
            }
            return Ok(s.to_string());
        }

        clipboard.get_text().map_err(|e| match e {
            ClipboardError::Unavailable(msg) => PlaceholderError::ClipboardUnavailable(msg),
            other => PlaceholderError::ClipboardUnavailable(other.to_string()),
        })
    }
}

struct ContextProcessor;

impl PlaceholderProcessor for ContextProcessor {
    fn name(&self) -> &str {
        "context"
    }

    fn description(&self) -> &str {
        "Persistent context data set across prompt executions"
    }

    fn process(
        &self,
        _context_override: Option<&str>,
        _clipboard: &ClipboardService,
        context_mgr: &ContextManagerService,
    ) -> Result<String, PlaceholderError> {
        Ok(context_mgr.get_context_or_default(""))
    }
}

pub struct PlaceholderService {
    processors: HashMap<String, Box<dyn PlaceholderProcessor + Send + Sync>>,
}

impl PlaceholderService {
    pub fn new() -> Self {
        let mut service = Self {
            processors: HashMap::new(),
        };
        service.register_processor(Box::new(ClipboardProcessor));
        service.register_processor(Box::new(ContextProcessor));
        service
    }

    pub fn register_processor(&mut self, processor: Box<dyn PlaceholderProcessor + Send + Sync>) {
        self.processors
            .insert(processor.name().to_string(), processor);
    }

    pub fn unregister_processor(&mut self, name: &str) {
        self.processors.remove(name);
    }

    pub fn process_messages(
        &self,
        messages: &[(&str, &str)],
        context_override: Option<&str>,
        clipboard: &ClipboardService,
        context_mgr: &ContextManagerService,
    ) -> Result<Vec<ProcessedMessage>, PlaceholderError> {
        let mut processed = Vec::new();

        for (i, (role, content)) in messages.iter().enumerate() {
            let processed_text =
                self.process_content(content, context_override, clipboard, context_mgr)?;

            let is_last = i == messages.len() - 1;
            let message_content = if is_last && context_mgr.has_images() {
                let mut parts = Vec::new();

                if !processed_text.trim().is_empty() {
                    parts.push(ContentPart::Text {
                        text: processed_text,
                    });
                }

                for item in &context_mgr.get_items() {
                    if let ContextItem::Image { data, media_type } = item {
                        parts.push(ContentPart::ImageUrl {
                            image_url: ImageUrlData {
                                url: format!("data:{media_type};base64,{data}"),
                            },
                        });
                    }
                }

                MessageContent::Parts(parts)
            } else {
                MessageContent::Text(processed_text)
            };

            processed.push(ProcessedMessage {
                role: role.to_string(),
                content: message_content,
            });
        }

        Ok(processed)
    }

    pub fn process_content(
        &self,
        content: &str,
        context_override: Option<&str>,
        clipboard: &ClipboardService,
        context_mgr: &ContextManagerService,
    ) -> Result<String, PlaceholderError> {
        let mut result = content.to_string();

        for (name, processor) in &self.processors {
            let pattern = format!("{{{{{name}}}}}");
            if result.contains(&pattern) {
                match processor.process(context_override, clipboard, context_mgr) {
                    Ok(value) => {
                        result = result.replace(&pattern, &value);
                    }
                    Err(PlaceholderError::ClipboardUnavailable(msg)) => {
                        return Err(PlaceholderError::ClipboardUnavailable(msg));
                    }
                    Err(e) => {
                        warn!("Placeholder processor '{name}' failed: {e}");
                        result = result.replace(&pattern, "");
                    }
                }
            }
        }

        Ok(result)
    }

    pub fn has_placeholders(&self, content: &str) -> bool {
        self.processors
            .keys()
            .any(|name| content.contains(&format!("{{{{{name}}}}}")))
    }

    pub fn find_invalid_placeholders(&self, content: &str) -> Vec<String> {
        let re = Regex::new(r"\{\{(\w+)\}\}").expect("invalid regex");
        re.captures_iter(content)
            .filter_map(|cap| {
                let name = cap[1].to_string();
                if self.processors.contains_key(&name) {
                    None
                } else {
                    Some(name)
                }
            })
            .collect()
    }

    pub fn get_placeholder_info(&self) -> HashMap<String, String> {
        self.processors
            .iter()
            .map(|(name, proc)| (name.clone(), proc.description().to_string()))
            .collect()
    }

    pub fn get_available_placeholders(&self) -> Vec<String> {
        self.processors.keys().cloned().collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    struct MockClipboardProcessor {
        value: String,
    }

    impl PlaceholderProcessor for MockClipboardProcessor {
        fn name(&self) -> &str {
            "clipboard"
        }

        fn description(&self) -> &str {
            "Mock clipboard"
        }

        fn process(
            &self,
            context_override: Option<&str>,
            _clipboard: &ClipboardService,
            _context_mgr: &ContextManagerService,
        ) -> Result<String, PlaceholderError> {
            if let Some(s) = context_override {
                if s.trim().is_empty() {
                    return Err(PlaceholderError::ClipboardUnavailable(
                        "Clipboard is empty".into(),
                    ));
                }
                return Ok(s.to_string());
            }
            if self.value.is_empty() {
                return Err(PlaceholderError::ClipboardUnavailable(
                    "Clipboard is empty".into(),
                ));
            }
            Ok(self.value.clone())
        }
    }

    struct FailingProcessor;

    impl PlaceholderProcessor for FailingProcessor {
        fn name(&self) -> &str {
            "failing"
        }

        fn description(&self) -> &str {
            "Always fails"
        }

        fn process(
            &self,
            _context_override: Option<&str>,
            _clipboard: &ClipboardService,
            _context_mgr: &ContextManagerService,
        ) -> Result<String, PlaceholderError> {
            Err(PlaceholderError::ProcessorFailed(
                "failing".into(),
                "test error".into(),
            ))
        }
    }

    fn test_service(clipboard_value: &str) -> PlaceholderService {
        let mut service = PlaceholderService {
            processors: HashMap::new(),
        };
        service.register_processor(Box::new(MockClipboardProcessor {
            value: clipboard_value.to_string(),
        }));
        service.register_processor(Box::new(ContextProcessor));
        service
    }

    fn dummy_clipboard() -> ClipboardService {
        ClipboardService::without_app()
    }

    #[test]
    fn text_substitution_clipboard() {
        let svc = test_service("pasted text");
        let ctx = ContextManagerService::new();
        let result = svc
            .process_content("Hello {{clipboard}}", None, &dummy_clipboard(), &ctx)
            .unwrap();
        assert_eq!(result, "Hello pasted text");
    }

    #[test]
    fn text_substitution_context() {
        let svc = test_service("");
        let mut ctx = ContextManagerService::new();
        ctx.set_context("my context".into());
        let result = svc
            .process_content("Data: {{context}}", None, &dummy_clipboard(), &ctx)
            .unwrap();
        assert_eq!(result, "Data: my context");
    }

    #[test]
    fn both_placeholders() {
        let svc = test_service("clip");
        let mut ctx = ContextManagerService::new();
        ctx.set_context("ctx".into());
        let result = svc
            .process_content(
                "{{clipboard}} and {{context}}",
                None,
                &dummy_clipboard(),
                &ctx,
            )
            .unwrap();
        assert_eq!(result, "clip and ctx");
    }

    #[test]
    fn empty_clipboard_error() {
        let svc = test_service("");
        let ctx = ContextManagerService::new();
        let result =
            svc.process_content("{{clipboard}}", None, &dummy_clipboard(), &ctx);
        assert!(matches!(
            result,
            Err(PlaceholderError::ClipboardUnavailable(_))
        ));
    }

    #[test]
    fn empty_context_fallback() {
        let svc = test_service("clip");
        let ctx = ContextManagerService::new();
        let result = svc
            .process_content("ctx={{context}}", None, &dummy_clipboard(), &ctx)
            .unwrap();
        assert_eq!(result, "ctx=");
    }

    #[test]
    fn context_override_used_for_clipboard() {
        let svc = test_service("ignored");
        let ctx = ContextManagerService::new();
        let messages = vec![("user", "{{clipboard}}")];
        let result = svc
            .process_messages(&messages, Some("override text"), &dummy_clipboard(), &ctx)
            .unwrap();
        assert_eq!(
            result[0].content,
            MessageContent::Text("override text".into())
        );
    }

    #[test]
    fn empty_context_override_error() {
        let svc = test_service("ignored");
        let ctx = ContextManagerService::new();
        let result = svc.process_content(
            "{{clipboard}}",
            Some("  "),
            &dummy_clipboard(),
            &ctx,
        );
        assert!(matches!(
            result,
            Err(PlaceholderError::ClipboardUnavailable(_))
        ));
    }

    #[test]
    fn image_injection_last_message() {
        let svc = test_service("clip");
        let mut ctx = ContextManagerService::new();
        ctx.append_context_image("abc123".into(), "image/png".into());
        let messages = vec![("system", "system msg"), ("user", "{{clipboard}}")];
        let result = svc
            .process_messages(&messages, None, &dummy_clipboard(), &ctx)
            .unwrap();

        assert_eq!(result[0].content, MessageContent::Text("system msg".into()));

        match &result[1].content {
            MessageContent::Parts(parts) => {
                assert_eq!(parts.len(), 2);
                assert!(matches!(&parts[0], ContentPart::Text { text } if text == "clip"));
                assert!(
                    matches!(&parts[1], ContentPart::ImageUrl { image_url } if image_url.url == "data:image/png;base64,abc123")
                );
            }
            _ => panic!("expected Parts content for last message with images"),
        }
    }

    #[test]
    fn image_injection_skips_empty_text() {
        let svc = test_service("clip");
        let mut ctx = ContextManagerService::new();
        ctx.append_context_image("abc".into(), "image/png".into());
        let messages = vec![("user", "")];
        let result = svc
            .process_messages(&messages, None, &dummy_clipboard(), &ctx)
            .unwrap();

        match &result[0].content {
            MessageContent::Parts(parts) => {
                assert_eq!(parts.len(), 1);
                assert!(matches!(&parts[0], ContentPart::ImageUrl { .. }));
            }
            _ => panic!("expected Parts content"),
        }
    }

    #[test]
    fn no_image_injection_on_non_last_message() {
        let svc = test_service("clip");
        let mut ctx = ContextManagerService::new();
        ctx.append_context_image("abc".into(), "image/png".into());
        let messages = vec![("system", "sys"), ("user", "usr")];
        let result = svc
            .process_messages(&messages, None, &dummy_clipboard(), &ctx)
            .unwrap();

        assert!(matches!(result[0].content, MessageContent::Text(_)));
    }

    #[test]
    fn no_placeholders_passthrough() {
        let svc = test_service("clip");
        let ctx = ContextManagerService::new();
        let result = svc
            .process_content("plain text", None, &dummy_clipboard(), &ctx)
            .unwrap();
        assert_eq!(result, "plain text");
    }

    #[test]
    fn invalid_placeholder_detection() {
        let svc = test_service("clip");
        let invalid = svc.find_invalid_placeholders("{{unknown}} and {{also_bad}}");
        assert!(invalid.contains(&"unknown".to_string()));
        assert!(invalid.contains(&"also_bad".to_string()));
        assert_eq!(invalid.len(), 2);
    }

    #[test]
    fn valid_placeholder_not_flagged() {
        let svc = test_service("clip");
        let invalid = svc.find_invalid_placeholders("{{clipboard}} and {{context}}");
        assert!(invalid.is_empty());
    }

    #[test]
    fn has_placeholders_detection() {
        let svc = test_service("clip");
        assert!(svc.has_placeholders("use {{clipboard}} here"));
        assert!(!svc.has_placeholders("no placeholders here"));
    }

    #[test]
    fn non_clipboard_error_silently_replaces() {
        let mut svc = PlaceholderService {
            processors: HashMap::new(),
        };
        svc.register_processor(Box::new(FailingProcessor));
        let ctx = ContextManagerService::new();
        let result = svc
            .process_content("before {{failing}} after", None, &dummy_clipboard(), &ctx)
            .unwrap();
        assert_eq!(result, "before  after");
    }

    #[test]
    fn get_placeholder_info_returns_descriptions() {
        let svc = test_service("clip");
        let info = svc.get_placeholder_info();
        assert_eq!(info.len(), 2);
        assert!(info.contains_key("clipboard"));
        assert!(info.contains_key("context"));
    }

    #[test]
    fn get_available_placeholders_returns_names() {
        let svc = test_service("clip");
        let names = svc.get_available_placeholders();
        assert_eq!(names.len(), 2);
        assert!(names.contains(&"clipboard".to_string()));
        assert!(names.contains(&"context".to_string()));
    }

    #[test]
    fn register_and_unregister_processor() {
        let mut svc = test_service("clip");
        assert_eq!(svc.get_available_placeholders().len(), 2);

        svc.unregister_processor("clipboard");
        assert_eq!(svc.get_available_placeholders().len(), 1);
        assert!(!svc.has_placeholders("{{clipboard}}"));
    }

    #[test]
    fn new_registers_default_processors() {
        let svc = PlaceholderService::new();
        let names = svc.get_available_placeholders();
        assert!(names.contains(&"clipboard".to_string()));
        assert!(names.contains(&"context".to_string()));
    }

    #[test]
    fn message_content_partial_eq() {
        assert_eq!(
            MessageContent::Text("hello".into()),
            MessageContent::Text("hello".into())
        );
        assert_ne!(
            MessageContent::Text("hello".into()),
            MessageContent::Text("world".into())
        );
    }

    #[test]
    fn process_messages_multiple_messages_no_images() {
        let svc = test_service("clip");
        let mut ctx = ContextManagerService::new();
        ctx.set_context("ctx".into());
        let messages = vec![
            ("system", "You are helpful. Context: {{context}}"),
            ("user", "{{clipboard}}"),
        ];
        let result = svc
            .process_messages(&messages, None, &dummy_clipboard(), &ctx)
            .unwrap();

        assert_eq!(result.len(), 2);
        assert_eq!(
            result[0].content,
            MessageContent::Text("You are helpful. Context: ctx".into())
        );
        assert_eq!(result[1].content, MessageContent::Text("clip".into()));
    }

    #[test]
    fn no_recursive_processing() {
        let mut svc = PlaceholderService {
            processors: HashMap::new(),
        };

        struct SelfReferencing;

        impl PlaceholderProcessor for SelfReferencing {
            fn name(&self) -> &str {
                "echo"
            }

            fn description(&self) -> &str {
                "Returns its own placeholder pattern"
            }

            fn process(
                &self,
                _context_override: Option<&str>,
                _clipboard: &ClipboardService,
                _context_mgr: &ContextManagerService,
            ) -> Result<String, PlaceholderError> {
                Ok("{{echo}}".to_string())
            }
        }

        svc.register_processor(Box::new(SelfReferencing));

        let ctx = ContextManagerService::new();
        let result = svc
            .process_content("{{echo}}", None, &dummy_clipboard(), &ctx)
            .unwrap();
        assert_eq!(result, "{{echo}}");
    }
}
