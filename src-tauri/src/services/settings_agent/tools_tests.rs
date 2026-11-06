use serde_json::{json, Map, Value};

use super::operations::Operation;
use super::tools::{definitions, operation_schema};

fn branches() -> Vec<Value> {
    operation_schema()["anyOf"].as_array().unwrap().clone()
}

fn branch_op(branch: &Value) -> &str {
    branch["properties"]["op"]["enum"][0].as_str().unwrap()
}

fn required(schema: &Value) -> Vec<&str> {
    schema.get("required").and_then(Value::as_array).into_iter().flatten().filter_map(Value::as_str).collect()
}

fn filled(schema: &Value) -> Value {
    if let Some(first) = schema.get("enum").and_then(|values| values.get(0)) {
        return first.clone();
    }
    match schema.get("type").and_then(Value::as_str) {
        Some("string") => json!("x"),
        Some("integer") => json!(1),
        Some("boolean") => json!(false),
        Some("array") => json!([]),
        Some("object") => {
            let mut object = Map::new();
            for name in required(schema) {
                object.insert(name.to_string(), filled(&schema["properties"][name]));
            }
            Value::Object(object)
        }
        _ => json!(true),
    }
}

fn index_of(operation: &Operation) -> usize {
    match operation {
        Operation::SetSetting { .. } => 0,
        Operation::AddModel { .. } => 1,
        Operation::UpdateModel { .. } => 2,
        Operation::DeleteModel { .. } => 3,
        Operation::CreateSkill { .. } => 4,
        Operation::UpdateSkill { .. } => 5,
        Operation::DeleteSkill { .. } => 6,
        Operation::ReorderSkills { .. } => 7,
        Operation::WritePrompt { .. } => 8,
        Operation::AddMcpServer { .. } => 9,
        Operation::UpdateMcpServer { .. } => 10,
        Operation::DeleteMcpServer { .. } => 11,
        Operation::AddWebviewProvider { .. } => 12,
        Operation::UpdateWebviewProvider { .. } => 13,
        Operation::DeleteWebviewProvider { .. } => 14,
        Operation::SetShortcut { .. } => 15,
        Operation::AddEnvVar { .. } => 16,
        Operation::UpdateEnvVar { .. } => 17,
        Operation::RenameEnvVar { .. } => 18,
        Operation::DeleteEnvVar { .. } => 19,
    }
}

#[test]
fn op_enum_equals_names_and_branches_follow_them() {
    let schema = operation_schema();
    assert_eq!(schema["properties"]["op"]["enum"], json!(Operation::NAMES));
    let branches = branches();
    let branch_ops: Vec<&str> = branches.iter().map(branch_op).collect();
    assert_eq!(branch_ops, Operation::NAMES);
}

#[test]
fn names_cover_every_variant() {
    let mut indexes: Vec<usize> = branches()
        .iter()
        .map(|branch| {
            let operation: Operation = serde_json::from_value(filled(branch)).unwrap();
            assert_eq!(Operation::NAMES[index_of(&operation)], operation.name());
            index_of(&operation)
        })
        .collect();
    indexes.sort();
    assert_eq!(indexes, (0..Operation::NAMES.len()).collect::<Vec<_>>());
}

#[test]
fn every_branch_requires_exactly_the_fields_serde_requires() {
    for branch in branches() {
        let op = branch_op(&branch);
        let full = filled(&branch);
        let operation: Operation = serde_json::from_value(full.clone())
            .unwrap_or_else(|e| panic!("{op} with only required fields: {e}"));
        assert_eq!(operation.name(), op);
        for field in required(&branch) {
            let mut without = full.clone();
            without.as_object_mut().unwrap().remove(field);
            assert!(
                serde_json::from_value::<Operation>(without).is_err(),
                "{op} still deserializes without `{field}`"
            );
        }
    }
}

#[test]
fn propose_and_test_tools_embed_the_operation_schema() {
    for tool in definitions().iter().filter(|tool| tool.name != "settings_read" && tool.name != "settings_secret") {
        let items = &tool.input_schema["properties"]["operations"]["items"];
        assert_eq!(items["anyOf"].as_array().unwrap().len(), Operation::NAMES.len(), "{}", tool.name);
    }
}

fn assert_strict_objects(schema: &Value, at: &str) {
    if let Some(properties) = schema.get("properties").and_then(Value::as_object) {
        assert_eq!(schema["additionalProperties"], json!(false), "{at}");
        let mut keys: Vec<&str> = properties.keys().map(String::as_str).collect();
        let mut listed = required(schema);
        keys.sort();
        listed.sort();
        assert_eq!(keys, listed, "{at}");
        for (name, child) in properties {
            assert_strict_objects(child, &format!("{at}.{name}"));
        }
    }
    if let Some(items) = schema.get("items") {
        assert_strict_objects(items, &format!("{at}[]"));
    }
}

#[test]
fn read_and_secret_schemas_satisfy_the_strict_rules() {
    for tool in definitions().iter().filter(|tool| tool.name == "settings_read" || tool.name == "settings_secret") {
        let schema = Value::Object((*tool.input_schema).clone());
        assert_strict_objects(&schema, &tool.name);
    }
}

#[test]
fn read_filters_other_than_what_are_nullable() {
    let tool = definitions().into_iter().find(|tool| tool.name == "settings_read").unwrap();
    let properties = tool.input_schema["properties"].as_object().unwrap();
    assert!(properties["what"]["enum"].as_array().unwrap().contains(&json!("change_log")));
    for (name, schema) in properties.iter().filter(|(name, _)| *name != "what") {
        let types = schema["type"].as_array().unwrap_or_else(|| panic!("{name} is not nullable"));
        assert!(types.contains(&json!("null")), "{name}");
    }
    assert_eq!(properties["limit"]["type"], json!(["integer", "null"]));
}

#[test]
fn only_read_and_secret_carry_the_strict_marker() {
    for tool in definitions() {
        let marked = tool
            .meta
            .as_ref()
            .and_then(|meta| meta.get(crate::services::ai::tools::STRICT_META_KEY))
            == Some(&json!(true));
        assert_eq!(marked, tool.name == "settings_read" || tool.name == "settings_secret", "{}", tool.name);
    }
}
