use super::*;
use crate::models::settings::{ModelConfig, ModelType, Provider};
use std::fs;
use tempfile::TempDir;

fn test_model(id: &str, api_key: &str) -> ModelConfig {
    ModelConfig {
        id: id.to_string(),
        model: "test".to_string(),
        display_name: "Test".to_string(),
        model_type: ModelType::Text,
        provider: Some(Provider::default()),
        group: None,
        api_key: Some(api_key.to_string()),
        base_url: None,
        parameters: None,
        context_window_size: None,
        api_mode: None,
        capabilities: None,
        store: true,
    }
}

fn test_stt_model(id: &str, api_key: &str) -> ModelConfig {
    ModelConfig {
        id: id.to_string(),
        model: "whisper-1".to_string(),
        display_name: "STT".to_string(),
        model_type: ModelType::Stt,
        provider: None,
        group: None,
        api_key: Some(api_key.to_string()),
        base_url: None,
        parameters: None,
        context_window_size: None,
        api_mode: None,
        capabilities: None,
        store: true,
    }
}

fn setup_test_dir() -> TempDir {
    let dir = TempDir::new().unwrap();
    let default_json = include_str!("../../../resources/default_settings.json");
    fs::write(dir.path().join("settings.json"), default_json).unwrap();
    dir
}

#[test]
fn test_load_default_settings() {
    let dir = setup_test_dir();
    let service = ConfigService::load(dir.path(), None).expect("should load default settings");
    assert!(!service.settings().models.is_empty());
    assert!(service.settings().models.iter().any(|m| m.is_text()));
    assert!(service.settings().models.iter().any(|m| m.is_stt()));
    assert!(service.settings().surfaces.chat.generation.model_id.is_some());
    assert!(service.settings().surfaces.speech_to_text.model_id.is_some());
}

#[test]
fn test_resolve_stt_model_returns_stt_entry() {
    let dir = setup_test_dir();
    let service = ConfigService::load(dir.path(), None).expect("load");
    let stt = service.resolve_stt_model().expect("stt model should resolve");
    assert!(stt.is_stt());
    assert_eq!(
        service.settings().surfaces.speech_to_text.language.as_deref(),
        Some("pl")
    );
}

#[test]
fn test_save_and_reload() {
    let dir = setup_test_dir();
    let service = ConfigService::load(dir.path(), None).expect("load");
    service.save().expect("save");

    let mut service2 = ConfigService::load(dir.path(), None).expect("reload");
    assert_eq!(service.settings().models.len(), service2.settings().models.len());
    assert_eq!(service.settings().models[0].id, service2.settings().models[0].id);

    service2.reload().expect("reload method");
    assert_eq!(service.settings().models[0].id, service2.settings().models[0].id);
}

#[test]
fn test_env_ref_preserved_on_save() {
    let dir = setup_test_dir();
    let service = ConfigService::load(dir.path(), None).expect("load");
    service.save().expect("save");

    let content = fs::read_to_string(dir.path().join("settings.json")).unwrap();
    let saved: serde_json::Value = serde_json::from_str(&content).unwrap();
    let models = saved["models"].as_array().unwrap();
    let text_model = models.iter().find(|m| m["type"] == "text").unwrap();
    assert_eq!(text_model["api_key"].as_str().unwrap(), "${OPENAI_API_KEY}");
    let stt_model = models.iter().find(|m| m["type"] == "stt").unwrap();
    assert_eq!(stt_model["api_key"].as_str().unwrap(), "${OPENAI_API_KEY}");
}

#[test]
fn test_delete_model_clears_references() {
    let dir = setup_test_dir();
    let mut service = ConfigService::load(dir.path(), None).expect("load");
    service.add_model(test_stt_model("stt-to-delete", "${KEY}"));
    service.settings_mut().surfaces.speech_to_text.model_id = Some("stt-to-delete".to_string());

    service.delete_model("stt-to-delete");
    assert_eq!(service.settings().surfaces.speech_to_text.model_id, None);
}

#[test]
fn test_direct_api_key_preserved_on_save() {
    let dir = setup_test_dir();
    let mut service = ConfigService::load(dir.path(), None).expect("load");
    service.add_model(test_model("direct-model", "sk-direct-secret"));
    service.save().expect("save");

    let content = fs::read_to_string(dir.path().join("settings.json")).unwrap();
    let saved: serde_json::Value = serde_json::from_str(&content).unwrap();
    let models = saved["models"].as_array().unwrap();
    let direct = models.iter().find(|m| m["id"] == "direct-model").unwrap();
    assert_eq!(direct["api_key"].as_str().unwrap(), "sk-direct-secret");
}

#[test]
fn test_mutation_methods() {
    let dir = setup_test_dir();
    let mut service = ConfigService::load(dir.path(), None).expect("load");

    let new_model = test_model("new-model", "${KEY}");
    let initial_count = service.settings().models.len();
    service.add_model(new_model.clone());
    assert_eq!(service.settings().models.len(), initial_count + 1);

    let mut updated = new_model.clone();
    updated.display_name = "GPT-5 Updated".to_string();
    service.update_model("new-model", updated);
    assert_eq!(
        service.settings().models.iter().find(|m| m.id == "new-model").unwrap().display_name,
        "GPT-5 Updated"
    );

    let upsert = test_model("upsert-model", "${KEY}");
    let count_before = service.settings().models.len();
    service.update_model("upsert-model", upsert);
    assert_eq!(service.settings().models.len(), count_before + 1);

    service.delete_model("upsert-model");
    assert!(service.settings().models.iter().all(|m| m.id != "upsert-model"));

    service.update_setting("debug_mode", serde_json::Value::Bool(true));
    assert!(service.settings().debug_mode);
}

#[test]
fn test_first_run_creates_defaults_from_fallback() {
    let dir = TempDir::new().unwrap();
    let service = ConfigService::load(dir.path(), None).expect("should create defaults");
    assert!(dir.path().join("settings.json").exists());
    assert!(dir.path().join(".env").exists());
    assert!(!service.settings().models.is_empty());
}

#[test]
fn test_first_run_copies_from_resource() {
    let dir = TempDir::new().unwrap();
    let resource_dir = TempDir::new().unwrap();
    let resource_settings_dir = resource_dir.path().join("resources");
    fs::create_dir_all(&resource_settings_dir).unwrap();
    let default_json = include_str!("../../../resources/default_settings.json");
    fs::write(resource_settings_dir.join("default_settings.json"), default_json).unwrap();

    let service = ConfigService::load(dir.path(), Some(resource_dir.path())).expect("should load");
    assert!(dir.path().join("settings.json").exists());
    assert!(!service.settings().models.is_empty());
}

#[test]
fn test_first_run_env_template_created() {
    let dir = TempDir::new().unwrap();
    let _service = ConfigService::load(dir.path(), None).expect("should create defaults");
    let env_content = fs::read_to_string(dir.path().join(".env")).unwrap();
    assert!(env_content.contains("OPENAI_API_KEY"));
}

#[test]
fn test_existing_env_not_overwritten() {
    let dir = TempDir::new().unwrap();
    fs::create_dir_all(dir.path()).unwrap();
    fs::write(dir.path().join(".env"), "OPENAI_API_KEY=real_key\n").unwrap();
    let _service = ConfigService::load(dir.path(), None).expect("should create defaults");
    let env_content = fs::read_to_string(dir.path().join(".env")).unwrap();
    assert_eq!(env_content, "OPENAI_API_KEY=real_key\n");
}

#[test]
fn test_migrates_legacy_schema() {
    let dir = TempDir::new().unwrap();
    let legacy = r#"{
        "default_model": "model-a",
        "quick_action_default_model": "model-b",
        "speech_to_text_model": "stt-a",
        "conversation_title_model": "model-c",
        "conversation_title_prompt": "Make a title",
        "system_prompt": "Custom system",
        "about_me": "about_me.md",
        "environment_section": "environment_section.md",
        "stt_prompt": "stt_prompt.md",
        "selected_tools": ["web_search"],
        "models": [
            {
                "id": "model-a",
                "type": "text",
                "model": "gpt-4",
                "display_name": "GPT-4",
                "provider": "openai",
                "api_key": "${OPENAI_API_KEY}",
                "parameters": { "reasoning_effort": "medium" },
                "enabled_tools": ["web_search"]
            },
            {
                "id": "model-b",
                "type": "text",
                "model": "gpt-4",
                "display_name": "Quick",
                "provider": "openai",
                "api_key": "${OPENAI_API_KEY}"
            },
            {
                "id": "model-c",
                "type": "text",
                "model": "gpt-4",
                "display_name": "Title",
                "provider": "openai",
                "api_key": "${OPENAI_API_KEY}"
            },
            {
                "id": "stt-a",
                "type": "stt",
                "model": "whisper-1",
                "display_name": "STT",
                "api_key": "${OPENAI_API_KEY}",
                "language": "pl",
                "keyterms_file": "keyterms.txt",
                "no_verbatim": true
            }
        ]
    }"#;

    fs::write(dir.path().join("settings.json"), legacy).unwrap();
    let service = ConfigService::load(dir.path(), None).expect("load migrated");
    let s = service.settings();

    assert_eq!(s.surfaces.chat.generation.model_id.as_deref(), Some("model-a"));
    assert_eq!(s.surfaces.chat.generation.parameters.reasoning_effort.as_deref(), Some("medium"));
    assert_eq!(s.surfaces.chat.generation.enabled_tools, vec!["web_search"]);
    assert_eq!(s.prompt_base.system, "prompts/base/system.md");
    let migrated_system = fs::read_to_string(dir.path().join("prompts/base/system.md")).unwrap();
    assert_eq!(migrated_system, "Custom system");
    assert_eq!(s.prompt_base.about_me, "about_me.md");
    assert_eq!(s.prompt_base.environment, "environment_section.md");
    assert_eq!(s.prompt_base.input_format, "prompts/base/input_format.md");
    assert_eq!(s.surfaces.quick_actions.generation.model_id.as_deref(), Some("model-b"));
    assert_eq!(s.surfaces.title_generation.generation.model_id.as_deref(), Some("model-c"));
    assert_eq!(s.surfaces.title_generation.prompt, "prompts/surfaces/title_generation.md");
    let migrated_title = fs::read_to_string(dir.path().join("prompts/surfaces/title_generation.md")).unwrap();
    assert_eq!(migrated_title, "Make a title");
    assert_eq!(s.surfaces.speech_to_text.model_id.as_deref(), Some("stt-a"));
    assert_eq!(s.surfaces.speech_to_text.language.as_deref(), Some("pl"));
    assert_eq!(s.surfaces.speech_to_text.keyterms_file.as_deref(), Some("keyterms.txt"));
    assert_eq!(s.surfaces.speech_to_text.no_verbatim, Some(true));
    assert_eq!(s.surfaces.speech_to_text.prompt.as_deref(), Some("stt_prompt.md"));

    let text_a = s.models.iter().find(|m| m.id == "model-a").unwrap();
    assert_eq!(text_a.parameters.as_ref().unwrap().reasoning_effort, None);

    let saved = fs::read_to_string(dir.path().join("settings.json")).unwrap();
    let saved_json: serde_json::Value = serde_json::from_str(&saved).unwrap();
    assert!(saved_json.get("default_model").is_none());
    assert!(saved_json.get("system_prompt").is_none());
    assert!(saved_json.get("stt_prompt").is_none());
    assert!(saved_json.get("surfaces").is_some());
}

#[test]
fn test_migrate_minimal_legacy() {
    let dir = TempDir::new().unwrap();
    let legacy = r#"{
        "default_model": "m1",
        "models": [
            { "id": "m1", "type": "text", "model": "gpt-4", "display_name": "M1", "provider": "openai", "api_key": "${OPENAI_API_KEY}" },
            { "id": "stt-only", "type": "stt", "model": "whisper-1", "display_name": "STT", "api_key": "${OPENAI_API_KEY}" }
        ]
    }"#;
    fs::write(dir.path().join("settings.json"), legacy).unwrap();
    let service = ConfigService::load(dir.path(), None).expect("load");
    let s = service.settings();
    assert_eq!(s.surfaces.chat.generation.model_id.as_deref(), Some("m1"));
    assert_eq!(s.surfaces.quick_actions.generation.model_id.as_deref(), Some("m1"));
}

#[test]
fn test_migrate_chat_prompt_fields_into_prompt_base() {
    let dir = TempDir::new().unwrap();
    let legacy = r#"{
        "surfaces": {
            "chat": {
                "generation": { "model_id": "m1", "parameters": {}, "enabled_tools": [] },
                "system_prompt": "Custom system",
                "about_me": "about_me.md",
                "environment_section": "environment_section.md"
            },
            "speech_to_text": { "model_id": "stt-1" }
        },
        "models": [
            { "id": "m1", "type": "text", "model": "gpt-4", "display_name": "T", "provider": "openai", "api_key": "${OPENAI_API_KEY}" },
            { "id": "stt-1", "type": "stt", "model": "whisper-1", "display_name": "STT", "api_key": "${OPENAI_API_KEY}" }
        ]
    }"#;
    fs::write(dir.path().join("settings.json"), legacy).unwrap();
    let service = ConfigService::load(dir.path(), None).expect("load");
    let s = service.settings();

    assert_eq!(s.prompt_base.system, "prompts/base/system.md");
    let migrated_system = fs::read_to_string(dir.path().join("prompts/base/system.md")).unwrap();
    assert_eq!(migrated_system, "Custom system");
    assert_eq!(s.prompt_base.about_me, "about_me.md");
    assert_eq!(s.prompt_base.environment, "environment_section.md");
    assert_eq!(s.surfaces.chat.generation.model_id.as_deref(), Some("m1"));

    let saved = fs::read_to_string(dir.path().join("settings.json")).unwrap();
    let saved_json: serde_json::Value = serde_json::from_str(&saved).unwrap();
    let chat = saved_json["surfaces"]["chat"].as_object().unwrap();
    assert!(!chat.contains_key("system_prompt"));
    assert!(!chat.contains_key("about_me"));
    assert!(!chat.contains_key("environment_section"));
    let prompt_base = saved_json["prompt_base"].as_object().unwrap();
    assert_eq!(prompt_base["system"], "prompts/base/system.md");
    assert!(!prompt_base.contains_key("system_prompt"));
    assert!(!prompt_base.contains_key("environment_section"));
}

#[test]
fn test_rewrite_legacy_default_path_when_new_file_present() {
    let dir = TempDir::new().unwrap();
    fs::create_dir_all(dir.path().join("prompts/base")).unwrap();
    fs::write(dir.path().join("prompts/base/about_me.md"), "real content").unwrap();
    fs::write(dir.path().join("about_me.md"), "stale template").unwrap();
    let legacy = r#"{
        "prompt_base": {
            "system": "prompts/base/system.md",
            "about_me": "about_me.md",
            "environment": "prompts/base/environment.md",
            "input_format": "prompts/base/input_format.md"
        },
        "surfaces": {
            "chat": { "generation": { "model_id": "m1" } },
            "title_generation": { "prompt": "prompts/surfaces/title_generation.md" },
            "speech_to_text": { "prompt": "prompts/surfaces/speech_to_text.md" }
        },
        "models": [
            { "id": "m1", "type": "text", "model": "gpt-4", "display_name": "T", "provider": "openai", "api_key": "${OPENAI_API_KEY}" }
        ]
    }"#;
    fs::write(dir.path().join("settings.json"), legacy).unwrap();
    let service = ConfigService::load(dir.path(), None).expect("load");
    assert_eq!(
        service.settings().prompt_base.about_me,
        "prompts/base/about_me.md",
        "legacy default path should be rewritten when canonical file exists"
    );
    assert_eq!(service.read_prompt(PromptKind::AboutMe), "real content");
}

#[test]
fn test_legacy_default_path_preserved_when_no_canonical_file() {
    let dir = TempDir::new().unwrap();
    fs::write(dir.path().join("about_me.md"), "user content").unwrap();
    let legacy = r#"{
        "prompt_base": { "about_me": "about_me.md" },
        "surfaces": {
            "chat": { "generation": { "model_id": "m1" } },
            "speech_to_text": {}
        },
        "models": [
            { "id": "m1", "type": "text", "model": "gpt-4", "display_name": "T", "provider": "openai", "api_key": "${OPENAI_API_KEY}" }
        ]
    }"#;
    fs::write(dir.path().join("settings.json"), legacy).unwrap();
    let service = ConfigService::load(dir.path(), None).expect("load");
    assert_eq!(
        service.settings().prompt_base.about_me,
        "prompts/base/about_me.md",
        "rename should move flat about_me.md to canonical path"
    );
    assert_eq!(service.read_prompt(PromptKind::AboutMe), "user content");
}

#[test]
fn test_migrate_stt_only_legacy() {
    let dir = TempDir::new().unwrap();
    let legacy = r#"{
        "speech_to_text_model": "stt-1",
        "models": [
            { "id": "text-1", "type": "text", "model": "gpt-4", "display_name": "T", "provider": "openai", "api_key": "${OPENAI_API_KEY}" },
            { "id": "stt-1", "type": "stt", "model": "whisper-1", "display_name": "STT", "api_key": "${OPENAI_API_KEY}", "language": "en" }
        ]
    }"#;
    fs::write(dir.path().join("settings.json"), legacy).unwrap();
    let service = ConfigService::load(dir.path(), None).expect("load");
    let s = service.settings();
    assert_eq!(s.surfaces.speech_to_text.model_id.as_deref(), Some("stt-1"));
    assert_eq!(s.surfaces.speech_to_text.language.as_deref(), Some("en"));
}
