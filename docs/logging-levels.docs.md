# Log Levels Reference

When to use each level, with real log lines from the codebase.

## Error

Something failed, operation cannot continue.

```rust
log::error!("show_notification failed: {e}");            // services/notification_window.rs
log::error!("Failed to copy transcription to clipboard: {e}");   // services/speech/session.rs
```

```typescript
logError(`drain_pending_notifications failed: ${e}`);    // useNotificationDrain.svelte.ts
```

Use for: unrecoverable failures, service initialization errors, operations needing user intervention.

## Warn

Unexpected but the app continues.

```rust
log::warn!("invalid shortcut {}: {}", shortcut_str, e);   // services/hotkeys.rs
log::warn!("unhandled menu item: {item_id}");            // commands/menu.rs
```

Use for: recoverable errors, fallback paths, missing optional config, failed non-critical features.

## Info

High-level events confirming the app works.

```rust
log::info!("config loaded from {}", config_dir.display());   // setup/init.rs
log::info!(target: "app_lib::hotkey_handler", "reloaded {} global shortcuts", bindings.len());   // services/hotkeys.rs
```

Use for: lifecycle events, config loaded/saved, user-triggered actions (log action, not content), counts and durations.

## Debug

Useful during development, too noisy for production.

```rust
log::debug!("context menu: refreshing {} skills", skill_summaries.len());   // commands/menu.rs
log::debug!("execute_menu_item: id={item_id}, shift={shift_pressed}");      // commands/menu.rs
```

Use for: request metadata (IDs, sizes — not content), state transitions, branch decisions.

## Trace

Deep debugging only. Almost never enabled.

```rust
log::trace!("responses: SSE event len={}", data.len());   // services/ai/openai_responses.rs
```

Use for: function entry/exit in hot paths, raw protocol data. Enable only when hunting a specific bug.
