# Frontend (Svelte 5 + TypeScript)

## Layout

- `windows/<name>/` — one folder per window: `entry.ts` + `App.svelte`.
- `lib/components/shared/ui/` — presentational primitives; `shared/widgets/` — reusable composites with state or IPC; `features/<feature>/` — feature UI.
- `lib/stores/` — rune stores (`*.svelte.ts`); `lib/services/` — per-domain `invoke()` wrappers and event listeners; `lib/types/` — types mirroring Rust structs; `lib/utils/`, `lib/constants/`.
- `styles/theme.css` — CSS variables; `styles/global.css` — global styles.

## Conventions

### State Management

Use Svelte 5 runes, not legacy stores:

- `$state()` for reactive state
- `$state.raw()` for large objects to avoid proxy overhead
- `$derived()` for computed values
- `$effect()` for side effects (e.g., syncing with Tauri backend)

Store files use `.svelte.ts` extension to enable rune syntax outside components.

### Window entries

This is a multi-window Tauri app. Every window has its own HTML at the project root (`index.html` for `main`) and, except the static `palette-backdrop.html`, a matching folder under `src/windows/<window-name>/`:

- `entry.ts` — mounts the root component (`mount(App, { target: ... })`).
- `App.svelte` — root component for that window. Keep thin; pull feature UI from `lib/components/`.
- An inline `<style>` in a window's HTML loses to the bundled `global.css` in production (the bundled stylesheet is linked after it). A transparent window overrides the `html, body` background in its App root (`:global(html), :global(body) { background: transparent; }`). Exception: `image-preview.html` uses inline `!important`.

Adding a new window:

1. Create `<name>.html` at project root with `<script type="module" src="/src/windows/<name>/entry.ts">`.
2. Create `src/windows/<name>/{entry.ts, App.svelte}`.
3. Add the HTML to `vite.config.ts → rollupOptions.input`.
4. Add the window label to `src-tauri/capabilities/default.json → windows` (child webviews go in `webviews`).
5. Build the window in Rust (`src-tauri/src/setup/windows.rs` for windows created at startup); `tauri.conf.json` has `"windows": []`.

### Components

- One component per file, organized by feature domain.
- Dumb reusable primitives go in `lib/components/shared/ui/` (no state machines, no feature concepts; no file there imports stores, services or `invoke`).
- Smart shared composites (state + logic, but reusable across features) go in `lib/components/shared/widgets/`.
- Feature-specific UI lives under `lib/components/features/<feature>/`. No cross-feature imports — pull into `shared/widgets/` if a component is needed by two features.
- Keep components thin — business logic lives in stores and services.

### Services (IPC Layer)

- `lib/services/` has one file per domain (`settings.ts`, `history.ts`, `skills.ts`, …) wrapping `invoke()` with typed signatures.
- Components call services, not `invoke()`; this keeps the IPC boundary explicit. Preference, not enforced: direct calls exist in `ContextMenu.svelte`, `shared/widgets/ContextEditor.svelte`, `ContextEditorView.svelte`, `windows/provider-menu/App.svelte`, `windows/recording-widget/App.svelte` and `windows/conversation-dialog/App.svelte` (`get_dialog_init_params`).

### Types

- `lib/types/` mirrors Rust serializable structs; update it when adding a Rust command or struct.
- Mirrors use the default serde field names (snake_case), so the TypeScript fields are snake_case too. Exception: `types/toolResult.ts` is camelCase.
- The `$lib` alias resolves to `src/lib/` (configured in `vite.config.ts` and `tsconfig.json`).

### Services (History)

- `lib/stores/history.svelte.ts` — reactive store via `getHistoryStore()`. Listens to `"history-changed"` events. Call `init()` on mount, `destroy()` on teardown.

### Services (Settings)

- `lib/services/events.ts` — Tauri event listener helpers (e.g., `onSettingsChanged`, `onHistoryChanged`).
- `invoke()` command names are snake_case (the Rust fn name); parameter names are camelCase (e.g. `{ providerId }`).
