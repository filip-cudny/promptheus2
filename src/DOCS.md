# Frontend (Svelte 5 + TypeScript)

## Directory Structure

```
src/
├── main.ts                     # App entry point (mounts App.svelte)
├── App.svelte                  # Root component, layout shell
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
│   │   ├── settings.svelte.ts  # App configuration
│   │   └── notifications.svelte.ts
│   ├── services/               # Tauri IPC wrappers
│   │   ├── api.ts              # invoke() wrapper with typed commands
│   │   ├── clipboard.ts        # Clipboard read/write via Tauri
│   │   └── events.ts           # Tauri event listeners
│   ├── types/                  # TypeScript type definitions
│   │   └── index.ts            # Mirrors Rust serializable types
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

### Services (Settings)

- `lib/services/settings.ts` — typed wrappers for all settings Tauri commands.
- `lib/services/events.ts` — Tauri event listener helpers (e.g., `onSettingsChanged`).
- Invoke parameter names must match Rust command parameter names exactly (snake_case).
