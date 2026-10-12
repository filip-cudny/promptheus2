# UI Components

Reusable UI primitives shared across features. See [src/DOCS.md](../../../../DOCS.md) for general frontend conventions.

## Conventions

### Component patterns

- Files here are presentational: they import no stores, services or `invoke()`, only types from `$lib/types` and constants. Components that need state or IPC go in `shared/widgets/`.
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

- `confirmIcon` swaps the icon for 1.2s after click; `confirmed` (bindable) exposes that state. `size` defaults to `ICON_SIZE.md`.
- Borderless transparent button — no border, hover adds subtle background.
- Use `confirmIcon={Check}` for actions where the user needs visual feedback (copy, save). Omit for actions with immediate visible effect (navigate, regenerate).
- Icon sizes are defined in `$lib/constants/ui.ts` — never hardcode pixel values.

### CollapsibleSection

- Uses `bind:collapsed` for two-way binding — parent controls open/closed state.
- Header uses Lucide `ChevronRight`/`ChevronDown` icons for collapse indicator. The `headerLeft` snippet renders between the chevron and title (e.g., role badges). The `actions` snippet renders on the right side and stops click propagation (`hoverActions` / `actionsVisible` control when it shows). `headerClass` allows conditional styling of the header (e.g., highlight when content is present).
- Content area uses `svelte/transition` `slide` for smooth expand/collapse.

### ImageChipBar

- `images` is bindable; `onremove` overrides the default removal from the array; `onopen` is called when a thumbnail is opened.
- Renders nothing when `images` is empty.
- Chip label is `Image #{image.number ?? position + 1}`: the optional `number` of `ConversationImage` wins, the 1-based position is the fallback.
- Each chip is an `AttachmentChip` with a thumbnail from the base64 data URI (120 px, or 64 px with `variant="small"`). Delete removes the image from the bound array unless `onremove` is given.
- Import `ConversationImage` from `$lib/types/conversation`.

### ContextEditor

Lives in `shared/widgets/ContextEditor.svelte` (it calls `invoke("open_image_preview")` for `ImageChipBar`'s `onopen`), not here. Parents control persistence.

### CommandPalette

- `inputRef` is bindable so the consumer can call `.focus()`.
- Pure visual shell: shared frame (`var(--surface-floating-modal)` with `--surface-floating-modal-border`), input row, scrollable body, footer. Consumers render the list and footer keys via snippets.
- `variant="overlay"` is for in-window overlays (e.g. Cmd+K inside ConversationDialog) — adds the `--surface-scrim` backdrop with fade-in.
- `variant="window"` is for separate Tauri windows with transparent body (e.g. Cmd+P provider switcher) — keeps the scrim transparent so the host window itself acts as the backdrop. On Linux this variant uses a lighter shadow (`--shadow-lg-linux`) because GTK/X11 transparent windows render heavy shadows poorly; the overlay variant keeps `--shadow-lg` on every platform.
- Provides utility classes for consumers: `.palette-item`, `.palette-item.highlight`, `.palette-item-icon`, `.palette-item-name`, `.palette-empty`, `.palette-divider`, plus `.palette-footer kbd` styling.

### MarkdownRenderer

- `onopen` and `onsavesvg` are optional callbacks (open a link/image, save a Mermaid SVG).
- Uses `marked` + `highlight.js` via `$lib/utils/markdown.ts` to render markdown with syntax-highlighted code blocks.
- `renderMarkdown` always highlights code blocks (`highlight.js`, auto-detecting the language when none is given); the renderer's `isStreaming` only paces how much of `content` is shown, not highlighting.
- Code copy: each code block gets a "Copy" button with a `data-copy-index` attribute. Click handler uses event delegation on the container div to copy the raw code block content to clipboard.

