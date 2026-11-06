use std::sync::Arc;

use rmcp::model::Tool;
use serde_json::{json, Value};

pub const READ: &str = "settings_read";
pub const TEST: &str = "settings_test";
pub const PROPOSE_CHANGES: &str = "settings_propose_changes";
pub const SECRET: &str = "settings_secret";

fn tool(name: &'static str, description: &'static str, schema: Value) -> Tool {
    let Value::Object(schema) = schema else {
        unreachable!("tool schemas are objects");
    };
    Tool::new(name, description, Arc::new(schema))
}

fn operations_schema() -> Value {
    json!({
        "type": "array",
        "description": "Operations in execution order. Each is an object with an `op` field: set_setting, add_model, update_model, delete_model, create_skill, update_skill, delete_skill, reorder_skills, write_prompt, add_mcp_server, update_mcp_server, delete_mcp_server, add_webview_provider, update_webview_provider, delete_webview_provider, set_shortcut, add_env_var, update_env_var, rename_env_var, delete_env_var. A later operation may reference a model or skill added by an earlier one.",
        "items": {"type": "object", "properties": {"op": {"type": "string"}}, "required": ["op"]},
        "minItems": 1
    })
}

pub fn definitions() -> Vec<Tool> {
    vec![
        tool(
            READ,
            "Read the live settings. `values`: all settings; `models`: models (api_key shown as a ${VAR} reference or [redacted]); `skills`: name, description, model and offered_in per skill; `skill`: one skill's frontmatter and body (needs `name`); `prompt`: one prompt (needs `name`: system, about_you, environment, input_format, title_generation, speech_to_text, stt_keyterms); `missing_env`: ${VAR} references without a value; `env_vars`: listed environment variables with description, secret flag, whether set on this machine and where used; never values.",
            json!({
                "type": "object",
                "properties": {
                    "what": {"type": "string", "enum": ["values", "models", "skills", "skill", "prompt", "missing_env", "env_vars"]},
                    "name": {"type": "string", "description": "Skill slug or prompt name, for `skill` and `prompt`."}
                },
                "required": ["what"]
            }),
        ),
        tool(
            TEST,
            "Check operations without proposing them: validates them as a whole and runs the test of each model, skill and MCP server row on the settings as they would be after the operations. Shows nothing to the user and writes nothing.",
            json!({
                "type": "object",
                "properties": {"operations": operations_schema()},
                "required": ["operations"]
            }),
        ),
        tool(
            PROPOSE_CHANGES,
            "Propose all changes of one request as one card for the user to accept or reject. Validates and tests the operations first; on row errors returns them and shows no card. A new proposal replaces the previous one.",
            json!({
                "type": "object",
                "properties": {"operations": operations_schema()},
                "required": ["operations"]
            }),
        ),
        tool(
            SECRET,
            "Ask the user to enter a secret (API key, token) for the environment variable `name`. The name must first be listed with `add_env_var` in an applied proposal; then this tool asks for its value. The app shows a masked input; the value never reaches you. Returns the outcome: saved, cancelled, or exists (the variable already has a value and the user closed without saving). Reference the variable as ${NAME} in operations.",
            json!({
                "type": "object",
                "properties": {
                    "name": {"type": "string", "description": "Environment variable name, letters, digits and underscores."},
                    "purpose": {"type": "string", "description": "What the secret is for, shown to the user."}
                },
                "required": ["name", "purpose"]
            }),
        ),
    ]
}
