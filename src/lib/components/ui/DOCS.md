# UI Components

Reusable UI primitives shared across features. See [src/DOCS.md](../../../DOCS.md) for general frontend conventions.

## Directory Structure

```
ui/
├── NotificationToast.svelte    # Global toast overlay (fixed position, bottom-right)
├── CollapsibleSection.svelte   # Expandable/collapsible content container
├── ImageChipBar.svelte         # Horizontal row of image thumbnails with delete
└── MarkdownRenderer.svelte     # Renders markdown to HTML with syntax highlighting
```

## Conventions

### Component patterns

- Components import state from `$lib/stores/` and types from `$lib/types` — no direct `invoke()` calls.
- Scoped `<style>` blocks for all CSS — no global style leakage.
- Custom Svelte transitions defined inline when built-in transitions (`fade`, `fly`) don't match the required animation curve (e.g., capped opacity).

### CollapsibleSection

- Props: `title: string`, `collapsed: boolean` (bindable), optional `actions` snippet, required `children` snippet.
- Uses `bind:collapsed` for two-way binding — parent controls open/closed state.
- Header is a `<button>` with a rotating arrow indicator. The `actions` snippet renders alongside the title (e.g., save buttons) and stops click propagation to avoid toggling collapse.

### ImageChipBar

- Props: `images: ConversationImage[]` (bindable), `readonly: boolean`.
- Renders nothing when `images` is empty.
- Each chip shows a 40×40 thumbnail from the base64 data URI. Delete button removes the image from the bound array.
- Import `ConversationImage` from `$lib/types/conversation`.

### MarkdownRenderer

- Props: `content: string`, `isStreaming: boolean`.
- Uses `marked` + `highlight.js` via `$lib/utils/markdown.ts` to render markdown with syntax-highlighted code blocks.
- When `isStreaming` is true, syntax highlighting is skipped for performance — plain escaped code is rendered instead, and highlighting applies when streaming ends.
- Code copy: each code block gets a "Copy" button with a `data-copy-index` attribute. Click handler uses event delegation on the container div to copy the raw code block content to clipboard.

### Toast component

- `NotificationToast` is a global overlay mounted once in `App.svelte` — not instantiated per-notification.
- Notifications are driven by the `notifications` store; the component iterates the reactive array with a keyed `{#each}`.
- Removal is triggered by the store's auto-dismiss timer; `onoutroend` calls `remove()` as a cleanup safety net after the out-transition completes.
- Visual config (icon colors, icons, opacity) is hardcoded as constants — will be wired to the settings store when available.
