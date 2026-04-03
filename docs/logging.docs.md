# Logging Guide

Comprehensive reference for logging in Promptheus. Covers log levels, sensitive data handling, environment configuration, and patterns for both Rust and TypeScript.

## Current Setup

Configured in `src-tauri/src/lib.rs` via `tauri-plugin-log`:

- **Targets**: stdout, log file (`LogDir`), webview console
- **Global level**: `Info`
- **App crate level**: `Debug` (module `app_lib`)
- **Timestamps**: local timezone
- **Override**: set `RUST_LOG` env var at runtime

Frontend calls `attachConsole()` in both `main.ts` and `context-menu-main.ts` to bridge Rust logs into browser devtools.

## Log Levels

### Error

Something failed and the operation cannot continue. Requires attention.

```rust
// Rust
log::error!("failed to load settings: {}", e);
log::error!("AI request failed for model {}: {}", model_id, e);
```

```typescript
// TypeScript
import { error } from "@tauri-apps/plugin-log";
error("Prompt execution failed: " + message);
```

**Use for:**
- Unrecoverable failures (file I/O, network, deserialization)
- Operations that need user intervention
- Service initialization failures

### Warn

Something unexpected happened but the app can continue.

```rust
log::warn!("model '{}' unavailable: {}", model.id, e);
log::warn!("invalid shortcut {}: {}", shortcut_str, e);
log::warn!("prompt migration failed: {}", e);
```

**Use for:**
- Recoverable errors or fallback paths taken
- Missing optional configuration
- Deprecated usage
- Failed registration of non-critical features (e.g., a shortcut)

### Info

High-level events that confirm the app is working as expected.

```rust
log::info!("config loaded from {}", config_dir.display());
log::info!("system tray initialized");
log::info!("registered shortcut: {} -> {}", shortcut_str, action);
log::info!("migrated {} prompts to skills", migrated.len());
```

**Use for:**
- App lifecycle (startup, shutdown, window creation)
- Configuration loaded/saved
- Feature initialization
- User-triggered actions (prompt executed, shortcut fired) — log the action, not the content
- Aggregated results (counts, durations)

### Debug

Detailed information useful during development but too noisy for production.

```rust
log::debug!("completion request: model={}, messages={}", model_id, msg_count);
log::debug!("stream chunk received: {} bytes", chunk.len());
log::debug!("config key changed: {}", key_name);
```

```typescript
import { debug } from "@tauri-apps/plugin-log";
debug("Store updated: " + store_name);
```

**Use for:**
- Request/response metadata (IDs, sizes, counts — not content)
- Internal state transitions
- Intermediate computation results
- Branch decisions in complex logic

### Trace

Extremely verbose, for deep debugging of specific issues. Almost never enabled.

```rust
log::trace!("entering process_placeholder with {} segments", segments.len());
log::trace!("SSE line: {:?}", line_type);
```

**Use for:**
- Function entry/exit in hot paths
- Raw protocol-level data (SSE lines, IPC messages)
- Iteration details in loops
- Enabled only when hunting a specific bug

## Data Sensitivity Classification

### NEVER log (any level)

| Data type | Examples | Why |
|-----------|----------|-----|
| API keys / tokens | OpenAI key, auth tokens | Credential theft |
| User message content | Prompt text, AI responses, chat history | Privacy |
| Clipboard content | Pasted text, copied data | May contain passwords, PII |
| Image data | Base64 strings, raw image bytes | Large + may contain sensitive info |
| File contents | User documents read as context | Privacy |
| Environment variable values | `.env` values | May contain secrets |

If you must reference these, log only metadata:

```rust
// WRONG
log::debug!("clipboard content: {}", clipboard_text);
log::debug!("API key: {}", api_key);

// RIGHT
log::debug!("clipboard read: {} chars", clipboard_text.len());
log::debug!("API key source: {:?}", key_source);
```

### Log with care (debug level only)

| Data type | What to log | What to omit |
|-----------|-------------|--------------|
| AI requests | model ID, message count, parameters | message content |
| Prompt execution | prompt/skill ID, status, duration | prompt text, result text |
| Config changes | which key changed | new value (if sensitive) |
| HTTP requests | URL path, status code, latency | headers, body |
| Error messages from APIs | error code, status | response body (may echo user input) |

```rust
// AI request — log metadata, not content
log::debug!(
    "completion: model={}, messages={}, max_tokens={}",
    model_id, messages.len(), params.max_tokens
);

// Prompt execution — log status, not text
log::info!("skill {} executed in {:.1}s, {} tokens", skill_id, elapsed, token_count);

// Config — log key name, not value
log::debug!("setting updated: {}", setting_key);
```

### Safe to log (any level)

| Data type | Examples |
|-----------|----------|
| Lifecycle events | app started, window created, tray initialized |
| Operation status | success, failure, skipped |
| Counts and sizes | message count, token count, byte size |
| Timing | duration of operations |
| IDs and names | model ID, skill ID, shortcut string |
| Paths | config directory, log directory (not file contents) |
| Feature flags | which features are enabled/disabled |

## Environment Configuration

### Development (debug build)

The current setup is good for development — `Debug` level for app code, `Info` for everything else, all three targets enabled.

To get more verbose output temporarily:

```bash
RUST_LOG=trace pnpm tauri dev          # everything at trace
RUST_LOG=app_lib=trace pnpm tauri dev  # only app code at trace
RUST_LOG=app_lib=trace,hyper=warn pnpm tauri dev  # fine-grained
```

### Production (release build)

For production builds, consider tightening the configuration:

```rust
let base_level = if cfg!(debug_assertions) {
    log::LevelFilter::Debug
} else {
    log::LevelFilter::Info
};

tauri_plugin_log::Builder::new()
    .targets([
        // No Stdout in production — users don't launch from terminal
        #[cfg(debug_assertions)]
        tauri_plugin_log::Target::new(tauri_plugin_log::TargetKind::Stdout),
        tauri_plugin_log::Target::new(tauri_plugin_log::TargetKind::LogDir {
            file_name: None,
        }),
        // Webview only in dev — no console.log noise in production
        #[cfg(debug_assertions)]
        tauri_plugin_log::Target::new(tauri_plugin_log::TargetKind::Webview),
    ])
    .level(base_level)
    .level_for("app_lib", if cfg!(debug_assertions) {
        log::LevelFilter::Debug
    } else {
        log::LevelFilter::Info
    })
    // Silence noisy library crates
    .level_for("hyper", log::LevelFilter::Warn)
    .level_for("reqwest", log::LevelFilter::Warn)
    .level_for("tao", log::LevelFilter::Warn)
    .build()
```

### Runtime override

`RUST_LOG` env var always works for runtime override, regardless of build type. Useful for debugging production issues without rebuilding.

## Log Targets and Rotation

### File location

`tauri-plugin-log` with `LogDir` writes to the platform-specific log directory:

| Platform | Path |
|----------|------|
| macOS | `~/Library/Logs/{bundleIdentifier}/` |
| Linux | `~/.config/{bundleIdentifier}/` |
| Windows | `%APPDATA%/{bundleIdentifier}/logs/` |

### Rotation

Configure rotation to prevent unbounded disk growth:

```rust
tauri_plugin_log::Builder::new()
    .max_file_size(10_000_000) // 10 MB per file
    .rotation_strategy(tauri_plugin_log::RotationStrategy::KeepAll)
    // ...
```

**Strategies:**
- `KeepAll` — keep all rotated files (use with manual cleanup)
- `KeepLastDays(n)` — auto-delete logs older than N days
- `DeleteOnStartup` — fresh log each launch (simplest, loses history)

**Recommendation for Promptheus:** `KeepAll` with `max_file_size(10_000_000)`. Desktop apps run intermittently, so log volume is modest. Users can manually clear old logs if needed.

## Performance

### Avoid expensive formatting in disabled levels

The `log` crate macros short-circuit when the level is disabled — the format string is never evaluated. This is already efficient:

```rust
// This is fine — no cost when debug is disabled
log::debug!("processed {} items in {:.2}s", items.len(), elapsed);
```

But avoid expensive computations inside the macro:

```rust
// BAD — expensive_summary() runs even if trace is disabled
log::trace!("state: {}", expensive_summary(&state));

// GOOD — guard with level check
if log::log_enabled!(log::Level::Trace) {
    log::trace!("state: {}", expensive_summary(&state));
}
```

### Async logging

`tauri-plugin-log` handles async file writes automatically. Log calls in Rust are non-blocking.

Frontend `info()`, `warn()`, `error()` from `@tauri-apps/plugin-log` are async (return `Promise`) — `await` them in sequential code, or fire-and-forget in event handlers where you don't need to wait.

### Hot paths

In high-frequency code (SSE stream processing, animation frames), prefer `trace!` level so logs are disabled by default. Never log per-character or per-byte in a stream.

```rust
// In SSE stream loop — trace level, not debug
log::trace!("SSE chunk: {} bytes", chunk.len());

// Log summary after stream completes — info level
log::info!("stream complete: {} chunks, {} tokens", chunk_count, token_count);
```

## Frontend Logging Patterns

### Setup

Both entry points already call `attachConsole()`. No additional setup needed.

### Imports

```typescript
import { error, warn, info, debug, trace } from "@tauri-apps/plugin-log";
```

### When to use frontend logging

| Scenario | Level | Example |
|----------|-------|---------|
| Invoke failure | `error` | `error("Failed to execute prompt: " + e)` |
| Unexpected UI state | `warn` | `warn("Store not initialized when expected")` |
| User action | `info` | `info("User triggered voice input")` |
| State update | `debug` | `debug("Execution store reset")` |

### Error handling pattern

```typescript
try {
    await invoke("execute_prompt", { skillId });
} catch (e) {
    error("execute_prompt failed: " + e);
    // show user-facing error
}
```

Do not log the full error object with `JSON.stringify` — it may contain user data echoed back in error messages. Log the error type/message only.

## Module-Specific Guidelines

| Module | Log level | What to log | What to redact |
|--------|-----------|-------------|----------------|
| **AI service** | info/warn | model ID, token count, latency, errors | message content, API keys |
| **Prompt execution** | info | skill ID, status, duration | prompt text, AI response |
| **Config** | info/debug | file paths, key names, validation errors | setting values for API keys |
| **Clipboard** | debug | operation type, char count | clipboard content |
| **Image storage** | debug | file path, byte size | image data |
| **Notification** | debug | notification type, delivery status | notification content if user-generated |
| **Skills** | info | skill loaded/executed, count | skill content |
| **Hotkeys** | info | shortcut string, action name | — (safe to log fully) |
| **Context menu** | debug | menu shown, item selected | — (safe to log fully) |

## Quick Reference

```
error!  → something broke, needs attention
warn!   → something unexpected, app continues
info!   → milestone, user action, lifecycle event
debug!  → internal detail, metadata, state change
trace!  → deep debugging, protocol-level data

Never log: secrets, user content, clipboard, images
Debug only: request metadata, IDs, config keys
Always safe: lifecycle, counts, durations, paths, status
```
