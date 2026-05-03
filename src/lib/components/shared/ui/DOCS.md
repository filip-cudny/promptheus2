# UI Components

Reusable UI primitives shared across features. See [src/DOCS.md](../../../DOCS.md) for general frontend conventions.

## Directory Structure

```
ui/
├── ActionIconButton.svelte     # Icon-only button with optional confirm feedback
├── CollapsibleSection.svelte   # Expandable/collapsible content container
├── CommandPalette.svelte       # Shared palette shell (scrim + modal + input + body + footer)
├── ContextEditor.svelte        # Textarea + image chips for editing context
├── ImageChipBar.svelte         # Horizontal row of image thumbnails with delete
└── MarkdownRenderer.svelte     # Renders markdown to HTML with syntax highlighting
```

## Conventions

### Component patterns

- Components import state from `$lib/stores/` and types from `$lib/types` — no direct `invoke()` calls.
- Scoped `<style>` blocks for all CSS — no global style leakage.
- Custom Svelte transitions defined inline when built-in transitions (`fade`, `fly`) don't match the required animation curve (e.g., capped opacity).

### Dynamic components

Render dynamic components using `$derived` + direct tag syntax (Svelte 5 runes mode):

```svelte
<script>
  let ActiveIcon = $derived(condition ? iconA : iconB);
</script>
<ActiveIcon size={16} />
```

For typing dynamic Lucide icon props, use `ComponentType<SvelteComponent<IconProps>>` (aliased as `LucideIcon` in `ActionIconButton`). This is the proper Svelte 5 way to type class-based components passed as props. Do not use `any` or `Component<>` — use `ComponentType<>` which handles both legacy and modern components.

### ActionIconButton

- Props: `icon` (Lucide component), optional `confirmIcon` (swaps to this icon for 1.2s after click), `onclick`, `title`, `disabled`, `size` (defaults to `ICON_SIZE.md`).
- Borderless transparent button — no border, hover adds subtle background.
- Use `confirmIcon={Check}` for actions where the user needs visual feedback (copy, save). Omit for actions with immediate visible effect (navigate, regenerate).
- Icon sizes are defined in `$lib/constants/ui.ts` — never hardcode pixel values.

### CollapsibleSection

- Props: `title: string`, `collapsed: boolean` (bindable), optional `headerClass: string`, optional `headerLeft` snippet, optional `actions` snippet, required `children` snippet.
- Uses `bind:collapsed` for two-way binding — parent controls open/closed state.
- Header uses Lucide `ChevronRight`/`ChevronDown` icons for collapse indicator. The `headerLeft` snippet renders between the chevron and title (e.g., role badges). The `actions` snippet renders on the right side and stops click propagation. `headerClass` allows conditional styling of the header (e.g., highlight when content is present).
- Content area uses `svelte/transition` `slide` for smooth expand/collapse.

### ImageChipBar

- Props: `images: ConversationImage[]` (bindable), `readonly: boolean`.
- Renders nothing when `images` is empty.
- Each chip shows a 40×40 thumbnail from the base64 data URI. Delete button removes the image from the bound array.
- Import `ConversationImage` from `$lib/types/conversation`.

### ContextEditor

- Props: `text: string` (bindable), `images: ConversationImage[]` (bindable), optional `readonly`, `disabled`, `placeholder`.
- Pure presentational component — no IPC calls or save logic. Parents control persistence.
- Renders `ImageChipBar` above a textarea. Used in both the conversation dialog's context section and the context menu's inline edit mode.
- Import `ConversationImage` from `$lib/types/conversation`.

### CommandPalette

- Props: `open: boolean`, `onClose: () => void`, `query: string` (bindable), `placeholder: string`, `variant: "overlay" | "window"`, optional `bodyMaxHeight` (default `"360px"`), optional `inputRef: HTMLInputElement | undefined` (bindable, lets the consumer call `.focus()`), optional `headerExtras` snippet, required `body` snippet, optional `footer` snippet.
- Pure visual shell: shared frame (`#252525`, 1px white-08 border, 8px radius), input row, scrollable body, footer. Consumers render the list and footer keys via snippets.
- `variant="overlay"` is for in-window overlays (e.g. Cmd+K inside ConversationDialog) — adds a dark `rgba(0,0,0,0.5)` scrim with fade-in.
- `variant="window"` is for separate Tauri windows with transparent body (e.g. Cmd+P provider switcher) — keeps the scrim transparent so the host window itself acts as the backdrop. On Linux this variant uses a lighter shadow (`0 6px 18px / 0.22`) because GTK/X11 transparent windows render heavy shadows poorly; the overlay variant keeps the heavy `0 20px 60px / 0.5` shadow on every platform.
- Provides utility classes for consumers: `.palette-item`, `.palette-item.highlight`, `.palette-item-icon`, `.palette-item-name`, `.palette-empty`, `.palette-divider`, plus `.palette-footer kbd` styling.

### MarkdownRenderer

- Props: `content: string`, `isStreaming: boolean`.
- Uses `marked` + `highlight.js` via `$lib/utils/markdown.ts` to render markdown with syntax-highlighted code blocks.
- When `isStreaming` is true, syntax highlighting is skipped for performance — plain escaped code is rendered instead, and highlighting applies when streaming ends.
- Code copy: each code block gets a "Copy" button with a `data-copy-index` attribute. Click handler uses event delegation on the container div to copy the raw code block content to clipboard.

