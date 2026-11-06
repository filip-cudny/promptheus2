use std::collections::HashMap;

use serde_json::{json, Value};
use tempfile::TempDir;

use super::operations::{Operation, PromptFile};
use super::plan::{plan, stale_rows, Current, Plan, RowError};
use crate::models::settings::{
    KeymapGroup, McpServerConfig, ModelConfig, ModelType, Provider, Settings, WebviewProvider,
};
use crate::models::skill::SkillFrontmatter;
use crate::services::change_log::{Effect, Target};

const OLD_SKILL: &str = "---\nname: old-skill\n---\n\nOld body.\n";

fn model(id: &str, model_type: ModelType, api_key: &str) -> ModelConfig {
    ModelConfig {
        id: id.to_string(),
        model: "m".to_string(),
        display_name: format!("Model {id}"),
        model_type,
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

fn fixture_settings() -> Settings {
    let mut settings = Settings::default();
    settings.models = vec![
        model("abc", ModelType::Text, "${OPENAI_KEY}"),
        model("def", ModelType::Text, "${OTHER_KEY}"),
        model("stt1", ModelType::Stt, "${STT_KEY}"),
    ];
    settings.surfaces.chat.generation.model_id = Some("abc".into());
    settings.surfaces.quick_actions.generation.model_id = Some("abc".into());
    settings.surfaces.title_generation.generation.model_id = Some("abc".into());
    settings.surfaces.speech_to_text.model_id = Some("stt1".into());
    settings.skills_order = vec!["old-skill".into()];
    settings.keymaps = vec![KeymapGroup {
        context: format!("os == {}", std::env::consts::OS),
        bindings: HashMap::from([("ctrl+f1".to_string(), "open_context_menu".to_string())]),
    }];
    settings.webview_providers = vec![provider("claude", "https://claude.ai/")];
    settings.mcp_servers = HashMap::from([("tools".to_string(), mcp("run-tools"))]);
    settings
}

fn provider(id: &str, url: &str) -> WebviewProvider {
    WebviewProvider { id: id.into(), name: format!("Provider {id}"), url: url.into() }
}

fn mcp(command: &str) -> McpServerConfig {
    McpServerConfig {
        command: command.into(),
        args: vec![],
        env: HashMap::new(),
        timeout_secs: 30,
        tool_timeouts: HashMap::new(),
    }
}

fn frontmatter(slug: &str) -> SkillFrontmatter {
    SkillFrontmatter {
        name: slug.into(),
        display_name: None,
        description: Some("d".into()),
        model: None,
        parameters: None,
        offered_in: None,
    }
}

fn config_dir() -> TempDir {
    let dir = TempDir::new().unwrap();
    let skill_dir = dir.path().join("skills/old-skill");
    std::fs::create_dir_all(&skill_dir).unwrap();
    std::fs::write(skill_dir.join("SKILL.md"), OLD_SKILL).unwrap();
    std::fs::write(skill_dir.join("notes.txt"), "notes").unwrap();
    dir
}

fn set_setting(key: &str, value: Value) -> Operation {
    Operation::SetSetting { key: key.into(), value }
}

fn plan_all(ops: &[Operation], settings: &Settings, dir: &TempDir) -> Result<Plan, Vec<RowError>> {
    plan(ops, &vec![true; ops.len()], &Current { settings, config_dir: dir.path() })
}

fn effects_of(plan: &Plan) -> Vec<&Effect> {
    plan.rows.iter().flat_map(|row| &row.operation.effects).collect()
}

fn setting_effect<'a>(plan: &'a Plan, path: &str) -> &'a Effect {
    effects_of(plan)
        .into_iter()
        .find(|e| e.target == Target::Setting { path: path.to_string() })
        .unwrap_or_else(|| panic!("no effect for {path}"))
}

#[test]
fn dependent_row_plans_when_selected_and_names_the_unchecked_row() {
    let (settings, dir) = (fixture_settings(), config_dir());
    let ops = vec![
        Operation::AddModel { model: model("x", ModelType::Text, "${X_KEY}") },
        set_setting("surfaces.chat.generation.model_id", json!("x")),
    ];
    let current = Current { settings: &settings, config_dir: dir.path() };

    let planned = plan(&ops, &[true, true], &current).expect("both selected");
    assert_eq!(planned.settings.surfaces.chat.generation.model_id.as_deref(), Some("x"));
    assert_eq!(planned.rows.len(), 2);

    let errors = plan(&ops, &[false, true], &current).unwrap_err();
    assert_eq!(errors.len(), 1);
    assert_eq!(errors[0].row, 1);
    assert!(errors[0].message.contains("depends on unchecked row 1"), "{}", errors[0].message);
}

#[test]
fn literal_api_key_is_a_row_error_and_env_reference_passes() {
    let (settings, dir) = (fixture_settings(), config_dir());

    let literal = vec![Operation::AddModel { model: model("x", ModelType::Text, "sk-secret") }];
    let errors = plan_all(&literal, &settings, &dir).unwrap_err();
    assert_eq!(errors[0].row, 0);
    assert_eq!(errors[0].key.as_deref(), Some("models[x].api_key"));
    assert_eq!(errors[0].message, "secrets only through the secret element");

    let reference = vec![Operation::AddModel { model: model("x", ModelType::Text, "${OPENAI_KEY}") }];
    plan_all(&reference, &settings, &dir).expect("reference passes");
}

#[test]
fn update_model_keeps_the_stored_key() {
    let (mut settings, dir) = (fixture_settings(), config_dir());
    settings.models[1].api_key = Some("sk-stored".into());
    let mut updated = model("def", ModelType::Text, "sk-stored");
    updated.display_name = "Renamed".into();

    let planned = plan_all(&[Operation::UpdateModel { id: "def".into(), model: updated }], &settings, &dir)
        .expect("stored key passes");

    assert_eq!(planned.settings.models[1].api_key.as_deref(), Some("sk-stored"));
    assert_eq!(planned.settings.models[1].display_name, "Renamed");
}

#[test]
fn out_of_range_setting_is_a_row_error_with_its_key() {
    let (settings, dir) = (fixture_settings(), config_dir());

    let errors = plan_all(&[set_setting("history_retention_days", json!(5000))], &settings, &dir).unwrap_err();

    assert_eq!(errors[0].row, 0);
    assert_eq!(errors[0].key.as_deref(), Some("history_retention_days"));
}

#[test]
fn delete_skill_effects_hold_the_full_file_content_as_before() {
    let (settings, dir) = (fixture_settings(), config_dir());

    let planned = plan_all(&[Operation::DeleteSkill { slug: "old-skill".into() }], &settings, &dir).unwrap();

    let skill = effects_of(&planned)
        .into_iter()
        .find(|e| e.target == Target::File { path: "skills/old-skill/SKILL.md".into() })
        .expect("SKILL.md effect");
    assert_eq!(skill.before, Some(json!(OLD_SKILL)));
    assert_eq!(skill.after, None);
    let notes = effects_of(&planned)
        .into_iter()
        .find(|e| e.target == Target::File { path: "skills/old-skill/notes.txt".into() })
        .expect("notes.txt effect");
    assert_eq!(notes.before, Some(json!("notes")));
    assert!(planned.settings.skills_order.is_empty());
    assert_eq!(planned.files["skills/old-skill/SKILL.md"], None);
}

#[test]
fn delete_skill_with_non_utf8_file_is_a_row_error() {
    let (settings, dir) = (fixture_settings(), config_dir());
    std::fs::write(dir.path().join("skills/old-skill/blob.bin"), [0xff, 0xfe, 0x00]).unwrap();

    let errors = plan_all(&[Operation::DeleteSkill { slug: "old-skill".into() }], &settings, &dir).unwrap_err();

    assert!(errors[0].message.contains("not valid UTF-8"), "{}", errors[0].message);
}

#[test]
fn delete_model_of_the_chat_default_yields_model_and_surface_effects() {
    let (settings, dir) = (fixture_settings(), config_dir());

    let planned = plan_all(&[Operation::DeleteModel { id: "abc".into() }], &settings, &dir).unwrap();

    let removed = setting_effect(&planned, "models[abc]");
    assert_eq!(removed.after, None);
    assert_eq!(removed.before.as_ref().unwrap()["index"], json!(0));
    let surface = setting_effect(&planned, "surfaces.chat.generation.model_id");
    assert_eq!(surface.before, Some(json!("abc")));
    assert_eq!(surface.after, Some(Value::Null));
}

#[test]
fn stale_rows_returns_only_the_row_whose_before_changed() {
    let (settings, dir) = (fixture_settings(), config_dir());
    let ops = vec![
        set_setting("theme", json!("light")),
        set_setting("theme", json!("dark")),
        set_setting("recent_apps_count", json!(7)),
    ];
    let planned = plan_all(&ops, &settings, &dir).unwrap();
    assert_eq!(stale_rows(&planned, &Current { settings: &settings, config_dir: dir.path() }), Vec::<usize>::new());

    let mut changed = settings.clone();
    changed.recent_apps_count = 99;

    assert_eq!(stale_rows(&planned, &Current { settings: &changed, config_dir: dir.path() }), vec![2]);
}

#[test]
fn stale_rows_detects_a_changed_file() {
    let (settings, dir) = (fixture_settings(), config_dir());
    let ops = vec![Operation::UpdateSkill {
        slug: "old-skill".into(),
        frontmatter: frontmatter("old-skill"),
        body: "New body.".into(),
        sample_input: "x".into(),
    }];
    let planned = plan_all(&ops, &settings, &dir).unwrap();
    let current = Current { settings: &settings, config_dir: dir.path() };
    assert!(stale_rows(&planned, &current).is_empty());

    std::fs::write(dir.path().join("skills/old-skill/SKILL.md"), "edited").unwrap();

    assert_eq!(stale_rows(&planned, &current), vec![0]);
}

#[test]
fn every_operation_type_plans_on_the_fixture() {
    let (settings, dir) = (fixture_settings(), config_dir());
    let operations = vec![
        set_setting("theme", json!("light")),
        Operation::AddModel { model: model("new", ModelType::Text, "${NEW_KEY}") },
        Operation::UpdateModel {
            id: "def".into(),
            model: ModelConfig { display_name: "Renamed".into(), ..model("def", ModelType::Text, "") },
        },
        Operation::DeleteModel { id: "def".into() },
        Operation::CreateSkill {
            slug: "fresh-skill".into(),
            frontmatter: frontmatter("fresh-skill"),
            body: "Body".into(),
            sample_input: "x".into(),
        },
        Operation::UpdateSkill {
            slug: "old-skill".into(),
            frontmatter: frontmatter("old-skill"),
            body: "Changed".into(),
            sample_input: "x".into(),
        },
        Operation::DeleteSkill { slug: "old-skill".into() },
        Operation::ReorderSkills { order: vec!["a-skill".into()] },
        Operation::WritePrompt { kind: PromptFile::System, content: "Be brief.".into() },
        Operation::WritePrompt { kind: PromptFile::SttKeyterms, content: "term".into() },
        Operation::AddMcpServer { name: "more".into(), config: mcp("run-more") },
        Operation::UpdateMcpServer { name: "tools".into(), config: mcp("run-other") },
        Operation::DeleteMcpServer { name: "tools".into() },
        Operation::AddWebviewProvider { provider: provider("chatgpt", "https://chatgpt.com/") },
        Operation::UpdateWebviewProvider { id: "claude".into(), provider: provider("claude", "https://claude.ai/new") },
        Operation::DeleteWebviewProvider { id: "claude".into() },
        Operation::SetShortcut { action: "clear_context".into(), shortcut: Some("ctrl+f2".into()) },
    ];

    for operation in operations {
        let name = operation.name();
        let planned = plan_all(std::slice::from_ref(&operation), &settings, &dir)
            .unwrap_or_else(|errors| panic!("{name}: {errors:?}"));
        assert_eq!(planned.rows.len(), 1, "{name}");
        assert_eq!(planned.rows[0].operation.op, name);
        assert!(!planned.rows[0].operation.effects.is_empty(), "{name} has no effects");
    }
}

#[test]
fn operation_deserializes_from_its_tagged_form_and_labels_itself() {
    let operation: Operation = serde_json::from_value(json!({
        "op": "set_setting",
        "key": "theme",
        "value": "light"
    }))
    .unwrap();
    assert_eq!(operation.label(), "Theme");

    let add: Operation = serde_json::from_value(json!({
        "op": "add_model",
        "model": serde_json::to_value(model("x", ModelType::Text, "")).unwrap()
    }))
    .unwrap();
    assert_eq!(add.label(), "Add model Model x");
}

#[test]
fn skill_errors_are_attributed_to_their_row() {
    let (settings, dir) = (fixture_settings(), config_dir());
    let ops = vec![
        Operation::CreateSkill {
            slug: "old-skill".into(),
            frontmatter: frontmatter("old-skill"),
            body: "b".into(),
            sample_input: "x".into(),
        },
        Operation::UpdateSkill {
            slug: "missing-skill".into(),
            frontmatter: frontmatter("missing-skill"),
            body: "b".into(),
            sample_input: "x".into(),
        },
        Operation::DeleteSkill { slug: "missing-skill".into() },
        Operation::CreateSkill {
            slug: "Bad Slug".into(),
            frontmatter: frontmatter("Bad Slug"),
            body: "b".into(),
            sample_input: "x".into(),
        },
        Operation::CreateSkill {
            slug: "other-skill".into(),
            frontmatter: frontmatter("different"),
            body: "b".into(),
            sample_input: "x".into(),
        },
    ];

    let errors = plan_all(&ops, &settings, &dir).unwrap_err();

    assert_eq!(errors.iter().map(|e| e.row).collect::<Vec<_>>(), vec![0, 1, 2, 3, 4]);
}
