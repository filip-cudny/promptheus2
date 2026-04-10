# Fix: Stop button missing during hanging tool calls + no backend cancellation

## Context

When the AI makes server-side tool calls (web_search via Responses API), the SSE stream can stall indefinitely while the API executes the search. The user sees a spinner with growing elapsed time but **no stop button** — they're stuck.

Two root causes:

1. **Orphaned tool calls hide the stop button.** If the stream ends (done/error) before `onToolCallDone` fires for every tool call, `is_executing` becomes `false` (hiding the stop button) but the tool call's `status` remains `"in_progress"` (keeping the spinner and timer alive). The UI enters an impossible state: visually executing, but no way to interact.

2. **No backend cancellation for conversation execution.** `execute_conversation_from_tree` passes `cancel_rx: None` to `run_stream_loop` (line 695 of `prompt_execution.rs`). Even if the stop button worked, it only resets frontend state — the backend stream keeps running. Skill execution has cancellation via `start_skill_execution` → `cancel_rx`, but conversation execution doesn't.

## Changes

### 1. Clean up orphaned tool calls on execution end (frontend)

**File:** `src/lib/stores/conversation.svelte.ts`

In both `onDone` and `onError` callbacks (and their reconnect equivalents), after clearing `tab.active_tool_calls`, also mark any remaining in-progress tool calls on `assistantNode` as completed/failed:

```typescript
// In onDone callback, after existing cleanup:
assistantNode.tool_calls = assistantNode.tool_calls.map((tc) =>
  tc.status === "in_progress"
    ? { ...tc, status: "completed", completed_at: new Date().toISOString() }
    : tc,
);

// In onError callback, after existing cleanup:
assistantNode.tool_calls = assistantNode.tool_calls.map((tc) =>
  tc.status === "in_progress"
    ? { ...tc, status: "failed", error: message, completed_at: new Date().toISOString() }
    : tc,
);
```

Same pattern in the `catch` block and the reconnect `onDone`/`onError` callbacks.

Also in `stopTabExecution`, mark tool calls on the assistant node as cancelled:
```typescript
lastNode.tool_calls = lastNode.tool_calls.map((tc) =>
  tc.status === "in_progress"
    ? { ...tc, status: "cancelled", completed_at: new Date().toISOString() }
    : tc,
);
```

### 2. Add backend cancellation for conversation execution (Rust)

**File:** `src-tauri/src/services/prompt_execution.rs`

Add a `cancel_sender` for live (conversation) executions, similar to skill execution:

- Add a `live_cancel_sender: Option<watch::Sender<bool>>` field
- In `start_live`, create a `watch::channel` and return the `Receiver`
- Add `cancel_live()` method that sends `true` on the sender
- Clean up in `clear_live()`

**File:** `src-tauri/src/commands/prompt_execution.rs`

- `start_live` now returns `(Arc<...>, watch::Receiver<bool>)`
- Pass the `cancel_rx` to `run_stream_loop` at line 695 (instead of `None`)
- Add/update a Tauri command `cancel_live_execution` that calls `cancel_live()`

**File:** `src-tauri/src/commands/mod.rs` — register the new command.

**File:** `src-tauri/capabilities/default.json` — add the new command if needed.

### 3. Frontend stop calls backend cancel (frontend)

**File:** `src/lib/stores/conversation.svelte.ts`

In `stopTabExecution`, invoke the backend cancel command:

```typescript
function stopTabExecution(tab: TabState): void {
  if (!tab.is_executing) return;
  invoke("cancel_live_execution").catch(() => {});
  // ... rest of existing cleanup
}
```

**File:** `src/lib/services/promptExecution.ts` — add `cancelLiveExecution` wrapper if needed.

## Verification

1. Start a conversation with web_search enabled, ask something that triggers multiple searches
2. While a search is running (spinner visible), verify the stop button appears
3. Click stop — verify the spinner stops, tool call shows as cancelled, and the backend stream is actually terminated (check Rust logs)
4. Also verify normal completion still works: all tool calls show checkmarks, stop button disappears, send button returns
5. Verify error case: if the stream errors mid-tool-call, tool calls are marked failed (not left spinning)
