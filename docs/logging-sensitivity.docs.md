# Log Data Sensitivity

Three tiers of data sensitivity for logging decisions.

## NEVER log (any level)

| Data | Examples |
|------|----------|
| API keys / tokens | OpenAI key, auth tokens |
| User message content | Prompt text, AI responses, chat history |
| Clipboard content | Pasted text, copied data |
| Image data | Base64 strings, raw bytes |
| File contents | User documents read as context |
| Env variable values | `.env` values |

Log only metadata instead:

```rust
// WRONG
log::debug!("clipboard content: {}", clipboard_text);
log::debug!("API key: {}", api_key);

// RIGHT
log::debug!("clipboard read: {} chars", clipboard_text.len());
log::debug!("API key source: {:?}", key_source);
```

## Debug level only

| Data | Log this | Omit this |
|------|----------|-----------|
| AI requests | model ID, message count, parameters | message content |
| Prompt execution | skill ID, status, duration | prompt text, result |
| Config changes | which key changed | new value (if sensitive) |
| HTTP requests | URL path, status, latency | headers, body |
| API errors | error code, status | response body (may echo user input) |

```rust
log::debug!("completion: model={}, messages={}, max_tokens={}", model_id, messages.len(), params.max_tokens);
log::info!("skill {} executed in {:.1}s, {} tokens", skill_id, elapsed, token_count);
log::debug!("setting updated: {}", setting_key);
```

## Safe to log (any level)

Lifecycle events, operation status (success/failure/skipped), counts and sizes, timing, IDs and names (model ID, skill ID, shortcut string), file paths (not contents), feature flags.

## Per-module summary

| Module | What to log | What to redact |
|--------|-------------|----------------|
| AI service | model ID, token count, latency, errors | message content, API keys |
| Prompt execution | skill ID, status, duration | prompt text, AI response |
| Config | file paths, key names, validation errors | values for API key settings |
| Clipboard | operation type, char count | clipboard content |
| Image storage | file path, byte size | image data |
| Notification | type, delivery status | user-generated content |
| Hotkeys | shortcut string, action name | — (safe) |
| Context menu | menu shown, item selected | — (safe) |
