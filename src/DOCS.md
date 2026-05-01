# Frontend (Svelte 5 + TypeScript)

## Directory Structure

```
src/
├── windows/                    # One folder per Tauri window — entry.ts + App.svelte
│   ├── main/                   # Default app window (index.html)
│   ├── context-menu/           # Borderless popup menu
│   ├── conversation-dialog/
│   ├── shell-toolbar/
│   ├── provider-menu/
│   ├── palette/
│   ├── context-editor/
│   ├── history-dialog/
│   ├── settings-dialog/
│   ├── image-preview/
│   ├── text-preview/
│   └── notification/
├── lib/
│   ├── components/             # UI components
│   │   ├── ui/                 # Reusable primitives (buttons, inputs, dialogs)
│   │   ├── menu/               # Context menu rendering
│   │   ├── prompt/             # Prompt execution dialog, conversation tabs
│   │   ├── settings/           # Settings panels
│   │   ├── history/            # History browser
│   │   └── context/            # Context editor
│   ├── stores/                 # Reactive state (Svelte 5 runes)
│   │   ├── menu.svelte.ts      # Menu items, selection state
│   │   ├── execution.svelte.ts # Execution state, streaming
│   │   ├── history.svelte.ts   # History entries
│   │   ├── context.svelte.ts   # Context items (text + images)
│   │   └── settings.svelte.ts  # App configuration
│   ├── services/               # Tauri IPC wrappers
│   │   ├── ai.ts               # AI completions (sync + streaming via Channel)
│   │   ├── settings.ts         # Settings CRUD
│   │   ├── context.ts          # Context items CRUD
│   │   ├── history.ts          # History entries CRUD
│   │   └── events.ts           # Tauri event listeners
│   ├── types/                  # TypeScript type definitions
│   │   ├── index.ts            # Re-exports all type modules + Settings, ModelConfig, etc.
│   │   ├── ai.ts               # StreamEvent, ProcessedMessage, ContentPart
│   │   ├── menu.ts             # MenuItem, MenuItemType
│   │   ├── execution.ts        # ErrorCode
│   │   ├── context.ts          # ContextItem
│   │   └── history.ts          # HistoryEntry, ConversationHistoryData, LastInteractionData
│   └── utils/                  # Helpers
│       ├── markdown.ts         # Markdown rendering
│       └── theme.ts            # Colors, spacing, styling tokens
└── styles/                     # Global CSS
```

## Conventions

### State Management

Use Svelte 5 runes, not legacy stores:

- `$state()` for reactive state
- `$state.raw()` for large objects to avoid proxy overhead
- `$derived()` for computed values
- `$effect()` for side effects (e.g., syncing with Tauri backend)

Store files use `.svelte.ts` extension to enable rune syntax outside components.

### Window entries

This is a multi-window Tauri app. Every window has its own HTML at the project root and a matching folder under `src/windows/<window-name>/`:

- `entry.ts` — mounts the root component (`mount(App, { target: ... })`).
- `App.svelte` — root component for that window. Keep thin; pull feature UI from `lib/components/`.

Adding a new window:

1. Create `<name>.html` at project root with `<script type="module" src="/src/windows/<name>/entry.ts">`.
2. Create `src/windows/<name>/{entry.ts, App.svelte}`.
3. Add the HTML to `vite.config.ts → rollupOptions.input`.
4. Add the window label to `src-tauri/capabilities/default.json → windows`.
5. Define the window in `src-tauri/tauri.conf.json` (or create it programmatically) with the same label.

### Components

- One component per file, organized by feature domain.
- Reusable UI primitives go in `lib/components/ui/`.
- Feature components (menu, prompt, settings) go in their own subdirectories.
- Keep components thin — business logic lives in stores and services.

### Services (IPC Layer)

- `lib/services/api.ts` wraps `invoke()` with typed function signatures.
- Frontend never calls `invoke()` directly from components — always through services.
- This keeps the IPC boundary explicit and testable.

### Types

- `lib/types/index.ts` mirrors Rust serializable structs.
- The Rust structs use default serde field names (snake_case), so TypeScript types also use snake_case.
- When adding a new Rust command/struct, update types here too.
- The `$lib` alias resolves to `src/lib/` (configured in `vite.config.ts` and `tsconfig.json`).

### Services (AI)

- `lib/services/ai.ts` — `complete()` for one-shot completions, `completeStream()` for streaming.
- Streaming uses Tauri 2 `Channel` from `@tauri-apps/api/core` — not global event listeners.
- `completeStream()` accepts a callbacks object (`onChunk`, `onDone`, `onError`). The Channel dispatches `StreamEvent` messages by their `event` tag.

### Services (History)

- `lib/services/history.ts` — typed wrappers for history Tauri commands (`getHistory`, `addHistoryEntry`, `addConversationEntry`, `updateConversationEntry`, `getLastInteraction`, `clearHistory`, `copyHistoryContent`).
- `lib/stores/history.svelte.ts` — reactive store via `getHistoryStore()`. Listens to `"history-changed"` events, exposes `entries`, `count`, `isEmpty`, `lastTextEntry`, `lastSpeechEntry`. Call `init()` on mount, `destroy()` on teardown.

### Services (Settings)

- `lib/services/settings.ts` — typed wrappers for all settings Tauri commands.
- `lib/services/events.ts` — Tauri event listener helpers (e.g., `onSettingsChanged`, `onHistoryChanged`).
- Invoke parameter names must match Rust command parameter names exactly (snake_case).
