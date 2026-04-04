use tauri::State;
use tokio::sync::Mutex;

use super::settings::AppState;
use crate::models::settings::Provider;
use crate::services::tokenizer;

#[tauri::command]
pub async fn count_tokens(
    text: String,
    provider: String,
    _state: State<'_, Mutex<AppState>>,
) -> Result<usize, String> {
    let provider = match provider.as_str() {
        "openai" => Provider::Openai,
        "anthropic" => Provider::Anthropic,
        "gemini" => Provider::Gemini,
        _ => Provider::Openai,
    };

    Ok(tokio::task::spawn_blocking(move || {
        tokenizer::count_tokens(&text, &provider)
    })
    .await
    .map_err(|e| e.to_string())?)
}
