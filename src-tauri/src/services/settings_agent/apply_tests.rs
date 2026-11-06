use std::collections::{BTreeMap, HashMap};
use std::path::Path;

use serde_json::{json, Value};
use tempfile::TempDir;

use super::apply::{apply, undo, ApplyError, Context, UndoOutcome};
use super::operations::{Operation, PromptFile};
use super::plan::{plan, plan_single, Current, Plan};
use super::plan_tests::{env_var, referencing_settings};
use crate::models::settings::{
    KeymapGroup, McpServerConfig, ModelConfig, ModelType, Provider, Settings, WebviewProvider,
};
use crate::models::skill::SkillFrontmatter;
use crate::services::ai::AiService;
use crate::services::change_log::{change_log_dir, load_all, EntryKind, Target};
use crate::services::config::ConfigService;
use crate::services::env_resolve::set_override;

const OLD_SKILL: &str = "---\nname: old-skill\n---\n\nOld body.\n";
const HOST: &str = "test-host";

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

fn setup() -> (TempDir, ConfigService) {
    setup_with(fixture_settings())
}

fn setup_with(settings: Settings) -> (TempDir, ConfigService) {
    let dir = TempDir::new().unwrap();
    let skill_dir = dir.path().join("skills/old-skill");
    std::fs::create_dir_all(&skill_dir).unwrap();
    std::fs::write(skill_dir.join("SKILL.md"), OLD_SKILL).unwrap();
    std::fs::write(skill_dir.join("notes.txt"), "notes").unwrap();
    std::fs::write(
        dir.path().join("settings.json"),
        serde_json::to_string_pretty(&settings).unwrap(),
    )
    .unwrap();
    let config = ConfigService::load(dir.path()).unwrap();
    (dir, config)
}

fn tree(dir: &Path) -> BTreeMap<String, Vec<u8>> {
    fn walk(root: &Path, dir: &Path, out: &mut BTreeMap<String, Vec<u8>>) {
        for entry in std::fs::read_dir(dir).unwrap().flatten() {
            let path = entry.path();
            if path.is_dir() {
                walk(root, &path, out);
            } else {
                let name = path.strip_prefix(root).unwrap().to_string_lossy().into_owned();
                out.insert(name, std::fs::read(&path).unwrap());
            }
        }
    }
    let mut out = BTreeMap::new();
    walk(dir, dir, &mut out);
    out.retain(|name, _| !name.starts_with("change-log"));
    out
}

fn log_files(dir: &Path) -> BTreeMap<String, Vec<u8>> {
    let log_dir = change_log_dir(dir);
    let mut files = BTreeMap::new();
    if let Ok(read) = std::fs::read_dir(&log_dir) {
        for entry in read.flatten() {
            files.insert(entry.file_name().to_string_lossy().into_owned(), std::fs::read(entry.path()).unwrap());
        }
    }
    files
}

fn make_plan(ops: &[Operation], config: &ConfigService, dir: &Path) -> Plan {
    plan(ops, &vec![true; ops.len()], &Current { settings: config.settings(), config_dir: dir })
        .unwrap_or_else(|errors| panic!("{errors:?}"))
}

fn run_apply(plan: &Plan, config: &mut ConfigService, dir: &Path) -> Result<String, ApplyError> {
    let settings = config.settings().clone();
    let log_dir = change_log_dir(dir);
    let mut ctx = Context {
        config,
        config_dir: dir,
        log_dir: &log_dir,
        host: HOST,
        request: Some("request".into()),
        reply: Some("reply".into()),
        tests: BTreeMap::new(),
    };
    apply(plan, &Current { settings: &settings, config_dir: dir }, &mut ctx)
}

fn run_undo(id: &str, force: bool, config: &mut ConfigService, dir: &Path) -> UndoOutcome {
    let log_dir = change_log_dir(dir);
    let mut ctx = Context {
        config,
        config_dir: dir,
        log_dir: &log_dir,
        host: HOST,
        request: None,
        reply: None,
        tests: BTreeMap::new(),
    };
    undo(id, force, &mut ctx).unwrap()
}

fn undone_id(outcome: UndoOutcome) -> String {
    match outcome {
        UndoOutcome::Undone { entry_id } => entry_id,
        other => panic!("expected Undone, got {other:?}"),
    }
}

fn set_setting(key: &str, value: Value) -> Operation {
    Operation::SetSetting { key: key.into(), value }
}

#[test]
fn a_failing_third_write_restores_the_first_two_and_writes_no_log() {
    let (dir, mut config) = setup();
    let ops = vec![
        set_setting("theme", json!("light")),
        Operation::WritePrompt { kind: PromptFile::System, content: "Be brief.".into() },
        Operation::CreateSkill {
            slug: "blocked".into(),
            frontmatter: frontmatter("blocked"),
            body: "Body".into(),
            sample_input: "x".into(),
        },
    ];
    let planned = make_plan(&ops, &config, dir.path());
    std::fs::write(dir.path().join("skills/blocked"), "in the way").unwrap();
    let start = tree(dir.path());

    let error = run_apply(&planned, &mut config, dir.path()).unwrap_err();

    assert!(matches!(error, ApplyError::WriteFailed { row: 2, .. }), "{error}");
    assert_eq!(tree(dir.path()), start);
    assert!(log_files(dir.path()).is_empty());
}

#[test]
fn a_stale_row_fails_apply_and_writes_nothing() {
    let (dir, mut config) = setup();
    let planned = make_plan(&[set_setting("theme", json!("light"))], &config, dir.path());
    config
        .try_mutate(|c| c.update_setting("theme", json!("light")))
        .unwrap();
    let start = tree(dir.path());

    let error = run_apply(&planned, &mut config, dir.path()).unwrap_err();

    assert!(matches!(&error, ApplyError::Stale(rows) if rows == &vec![0]), "{error}");
    assert_eq!(tree(dir.path()), start);
    assert!(log_files(dir.path()).is_empty());
}

fn every_operation() -> Vec<Operation> {
    vec![
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
        Operation::UpdateWebviewProvider {
            id: "claude".into(),
            provider: provider("claude", "https://claude.ai/new"),
        },
        Operation::DeleteWebviewProvider { id: "claude".into() },
        Operation::SetShortcut { action: "clear_context".into(), shortcut: Some("ctrl+f2".into()) },
    ]
}

#[test]
fn apply_then_undo_is_byte_equal_for_every_operation_type() {
    for operation in every_operation() {
        let name = operation.name();
        let (dir, mut config) = setup();
        let start = tree(dir.path());
        let planned = make_plan(std::slice::from_ref(&operation), &config, dir.path());

        let card_id = run_apply(&planned, &mut config, dir.path()).unwrap_or_else(|e| panic!("{name}: {e}"));
        assert_ne!(tree(dir.path()), start, "{name} changed nothing");
        let card_file = log_files(dir.path());
        let outcome = run_undo(&card_id, false, &mut config, dir.path());

        let undo_id = undone_id(outcome);
        assert_eq!(tree(dir.path()), start, "{name}");
        let files = log_files(dir.path());
        assert_eq!(files.len(), 2, "{name}");
        let card_name = format!("{card_id}.json");
        assert_eq!(files[&card_name], card_file[&card_name], "{name}");
        let entries = load_all(&change_log_dir(dir.path()));
        let undo_entry = entries.iter().find(|e| e.id == undo_id).unwrap();
        assert_eq!(undo_entry.kind, EntryKind::Undo);
        assert_eq!(undo_entry.undoes.as_deref(), Some(card_id.as_str()));
    }
}

#[test]
fn a_whole_card_of_every_operation_type_round_trips() {
    let (dir, mut config) = setup();
    let start = tree(dir.path());
    let ops = vec![
        Operation::AddModel { model: model("new", ModelType::Text, "${NEW_KEY}") },
        set_setting("theme", json!("light")),
        Operation::DeleteSkill { slug: "old-skill".into() },
        Operation::DeleteMcpServer { name: "tools".into() },
        Operation::DeleteWebviewProvider { id: "claude".into() },
        Operation::WritePrompt { kind: PromptFile::System, content: "Be brief.".into() },
    ];
    let planned = make_plan(&ops, &config, dir.path());

    let card_id = run_apply(&planned, &mut config, dir.path()).unwrap();
    undone_id(run_undo(&card_id, false, &mut config, dir.path()));

    assert_eq!(tree(dir.path()), start);
}

#[test]
fn an_added_model_set_as_chat_default_resolves_in_the_ai_service() {
    set_override("PHX_APPLY_TEST_KEY", Some("value".into()));
    let (dir, mut config) = setup();
    let ops = vec![
        Operation::AddModel { model: model("x", ModelType::Text, "${PHX_APPLY_TEST_KEY}") },
        set_setting("surfaces.chat.generation.model_id", json!("x")),
    ];
    let planned = make_plan(&ops, &config, dir.path());

    run_apply(&planned, &mut config, dir.path()).unwrap();

    let settings = config.settings();
    assert!(AiService::new(&settings.models).has_model("x"));
    assert_eq!(settings.surfaces.chat.generation.model_id.as_deref(), Some("x"));
}

#[test]
fn undo_after_a_manual_change_returns_conflicts_and_force_restores() {
    let (dir, mut config) = setup();
    let start = tree(dir.path());
    let planned = make_plan(&[set_setting("theme", json!("light"))], &config, dir.path());
    let card_id = run_apply(&planned, &mut config, dir.path()).unwrap();
    config
        .try_mutate(|c| c.update_setting("theme", json!("dark")))
        .unwrap();
    let changed = tree(dir.path());

    let outcome = run_undo(&card_id, false, &mut config, dir.path());

    let UndoOutcome::Conflicts { conflicts } = outcome else {
        panic!("expected conflicts");
    };
    assert_eq!(conflicts.len(), 1);
    assert_eq!(conflicts[0].target, Target::Setting { path: "theme".into() });
    assert_eq!(conflicts[0].expected_after, Some(json!("light")));
    assert_eq!(conflicts[0].current, Some(json!("dark")));
    assert_eq!(tree(dir.path()), changed);
    assert_eq!(log_files(dir.path()).len(), 1);

    undone_id(run_undo(&card_id, true, &mut config, dir.path()));

    assert_eq!(tree(dir.path()), start);
}

#[test]
fn undo_of_delete_skill_restores_the_full_skill_file() {
    let (dir, mut config) = setup();
    let planned = make_plan(&[Operation::DeleteSkill { slug: "old-skill".into() }], &config, dir.path());
    let card_id = run_apply(&planned, &mut config, dir.path()).unwrap();
    assert!(!dir.path().join("skills/old-skill").exists());

    undone_id(run_undo(&card_id, false, &mut config, dir.path()));

    let skill = dir.path().join("skills/old-skill");
    assert_eq!(std::fs::read_to_string(skill.join("SKILL.md")).unwrap(), OLD_SKILL);
    assert_eq!(std::fs::read_to_string(skill.join("notes.txt")).unwrap(), "notes");
    assert_eq!(config.settings().skills_order, vec!["old-skill".to_string()]);
}

#[test]
fn undo_of_the_undo_entry_reapplies_and_names_the_second_entry() {
    let (dir, mut config) = setup();
    let planned = make_plan(
        &[set_setting("theme", json!("light")), Operation::DeleteSkill { slug: "old-skill".into() }],
        &config,
        dir.path(),
    );
    let card_id = run_apply(&planned, &mut config, dir.path()).unwrap();
    let applied = tree(dir.path());
    let undo_id = undone_id(run_undo(&card_id, false, &mut config, dir.path()));

    let redo_id = undone_id(run_undo(&undo_id, false, &mut config, dir.path()));

    assert_eq!(tree(dir.path()), applied);
    assert_eq!(log_files(dir.path()).len(), 3);
    let entries = load_all(&change_log_dir(dir.path()));
    let redo = entries.iter().find(|e| e.id == redo_id).unwrap();
    assert_eq!(redo.kind, EntryKind::Undo);
    assert_eq!(redo.undoes.as_deref(), Some(undo_id.as_str()));
}

fn rename(name: &str, new_name: &str) -> Operation {
    Operation::RenameEnvVar { name: name.into(), new_name: new_name.into() }
}

#[test]
fn undo_of_a_rename_restores_name_previous_names_and_every_reference() {
    let mut settings = referencing_settings("PHX_APPLY_OLD");
    settings.env_vars[0].renamed_from = vec!["PHX_APPLY_ANCIENT".into()];
    let (dir, mut config) = setup_with(settings);
    let start = tree(dir.path());
    let before = serde_json::to_value(config.settings()).unwrap();
    let planned = make_plan(&[rename("PHX_APPLY_OLD", "PHX_APPLY_NEW")], &config, dir.path());

    let card_id = run_apply(&planned, &mut config, dir.path()).unwrap();
    let renamed = &config.settings().env_vars[1];
    assert_eq!(renamed.name, "PHX_APPLY_NEW");
    assert_eq!(renamed.renamed_from, vec!["PHX_APPLY_OLD", "PHX_APPLY_ANCIENT"]);
    undone_id(run_undo(&card_id, false, &mut config, dir.path()));

    assert_eq!(serde_json::to_value(config.settings()).unwrap(), before);
    assert_eq!(tree(dir.path()), start);
}

#[test]
fn a_separate_update_of_another_variable_does_not_conflict_with_the_rename_undo() {
    let (dir, mut config) = setup_with(referencing_settings("PHX_APPLY_A"));
    let start = tree(dir.path());
    let renamed = make_plan(&[rename("PHX_APPLY_A", "PHX_APPLY_A2")], &config, dir.path());
    let rename_id = run_apply(&renamed, &mut config, dir.path()).unwrap();
    let update = Operation::UpdateEnvVar {
        name: "PHX_PLAN_BYSTANDER".into(),
        description: "changed".into(),
        secret: true,
    };
    let updated = make_plan(&[update], &config, dir.path());
    run_apply(&updated, &mut config, dir.path()).unwrap();

    let outcome = run_undo(&rename_id, false, &mut config, dir.path());

    undone_id(outcome);
    let vars = &config.settings().env_vars;
    assert_eq!(vars[0].name, "PHX_APPLY_A");
    assert_eq!(vars[1].description, "changed");
    assert_ne!(tree(dir.path()), start);
}

#[test]
fn the_gui_path_writes_the_same_card_operations_as_the_agent_path() {
    let operation = || rename("PHX_APPLY_GUI_OLD", "PHX_APPLY_GUI_NEW");
    let operations_of = |through_gui: bool| {
        let (dir, mut config) = setup_with(referencing_settings("PHX_APPLY_GUI_OLD"));
        let current = Current { settings: config.settings(), config_dir: dir.path() };
        let planned = if through_gui {
            plan_single(operation(), &current).unwrap()
        } else {
            plan(&[operation()], &[true], &current).unwrap()
        };
        let id = run_apply(&planned, &mut config, dir.path()).unwrap();
        let entries = load_all(&change_log_dir(dir.path()));
        let entry = entries.into_iter().find(|e| e.id == id).unwrap();
        assert_eq!(entry.kind, EntryKind::Card);
        entry.operations
    };

    let (gui, agent) = (operations_of(true), operations_of(false));

    assert_eq!(gui.len(), agent.len());
    for (gui, agent) in gui.iter().zip(&agent) {
        assert_eq!(gui.label, agent.label);
        assert_eq!(gui.op, agent.op);
        assert_eq!(gui.effects, agent.effects);
        assert_eq!(gui.test, agent.test);
    }
}

#[test]
fn add_update_and_delete_env_var_each_round_trip_through_undo() {
    let mut settings = fixture_settings();
    settings.env_vars = vec![env_var("PHX_APPLY_EXISTING")];
    for operation in [
        Operation::AddEnvVar { name: "PHX_APPLY_ADDED".into(), description: "d".into(), secret: true },
        Operation::UpdateEnvVar { name: "PHX_APPLY_EXISTING".into(), description: "d".into(), secret: true },
        Operation::DeleteEnvVar { name: "PHX_APPLY_EXISTING".into() },
    ] {
        let name = operation.name();
        let (dir, mut config) = setup_with(settings.clone());
        let start = tree(dir.path());
        let planned = make_plan(std::slice::from_ref(&operation), &config, dir.path());

        let card_id = run_apply(&planned, &mut config, dir.path()).unwrap();
        assert_ne!(tree(dir.path()), start, "{name} changed nothing");
        undone_id(run_undo(&card_id, false, &mut config, dir.path()));

        assert_eq!(tree(dir.path()), start, "{name}");
    }
}
