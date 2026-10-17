# UI Components

Reusable UI primitives shared across features. See [src/DOCS.md](../../../../DOCS.md) for general frontend conventions.

## Conventions

### Component patterns

- Files here are presentational: they import no stores, services or `invoke()`. Besides types, constants, `$lib/utils` helpers and sibling `ui/` components they use only rendering libraries (`morphdom` and `mermaid` in `MarkdownRenderer`). Components that need state or IPC go in `shared/widgets/`; the exception is `ThinkingBlock`, which calls the Tauri plugin `opener` directly (open a link); saving a Mermaid SVG comes in through its `onsavesvg` prop.
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

- `confirmIcon` swaps the icon for `COPY_FEEDBACK_MS` (1.2 s) after click through `useCopyFeedback`; a repeated click restarts the window. `confirmed` is a read-only prop that also shows `confirmIcon` (used by `AssistantBubble` auto-copy). `size` defaults to `ICON_SIZE.md`.
- Copy feedback comes from `useCopyFeedback` (`$lib/utils/copyFeedback.svelte`) or `ActionIconButton confirmIcon`, never an own timer; the check keeps the button's normal text colour, no copied-state colour.
- Borderless transparent button — no border, hover adds subtle background.
- Use `confirmIcon={Check}` for actions where the user needs visual feedback (copy, save). Omit for actions with immediate visible effect (navigate, regenerate).
- Icon sizes are defined in `$lib/constants/ui.ts` — never hardcode pixel values.
- `active` marks a toggled-on button (tint of `--info`); `variant="danger"` turns the hover red.

### FilePathReveal

- `path`; renders a small `…` summary that reveals the file path in a popover anchored to the right edge of its container (the title row of an editor).

### HighlightedTextarea

- `bind:value`, `highlightPattern` (placeholder regex), `unknownTokenChecker`, `placeholder`, `disabled`, `minHeight`.
- Exports `insert(token)`: inserts at the caret, keeps the caret after the token and updates the bound `value`; callers use it through `bind:this`.

### Modal

- `title`, `width` (px, default 480), `closeHover` (default true; `false` drops the close button hover background), `onClose`, `children`. Backdrop, header with close button, panel; a mousedown on the backdrop or `Escape` calls `onClose`.
- The panel is a flex column; children (fields, footer) are laid out inside it. Focus is the caller's job (`onMount`).

### MenuItem

- `label`, `icon` and `shortcut` snippets, `active`, `activeStyle`, `disabled`, `onclick` (receives the `MouseEvent`). Rows inside `MenuList`.

### FloatingPanel

- Every dropdown uses it with `MenuList` + `MenuItem` rows (`ModelSelector`, `AttachMenu`, `SidebarMoreMenu`, `SelectDropdown`, the context menu panels); no component positions its own dropdown or handles its outside click.
- `visible`, `anchorEl`, `onclose` (called by a pointer-down outside the panel and the anchor), `position` (`"auto"` below when it fits; `"below"` forced; `"above"` preferred, falls back below when there is more room there), `align` (`"start"`/`"end"`: left or right edge of the anchor, only with `fitContent`), `fitContent` (content width, anchored to the anchor), `flush` (no padding, defaults to `fitContent`).
- Positioned once on open, `position: fixed`, z-index `--z-overlay`: it does not follow its anchor, so an owner that scrolls (sidebar list) closes it on scroll. A panel taller than the room on its side gets a `max-height` and scrolls inside.
- A `transform` or `backdrop-filter` on an ancestor makes that ancestor the containing block of the fixed panel.

### CollapsibleSection

- Uses `bind:collapsed` for two-way binding — parent controls open/closed state.
- Header uses Lucide `ChevronRight`/`ChevronDown` icons for collapse indicator. The `actions` snippet renders on the right side and stops click propagation. `headerClass` allows conditional styling of the header (e.g., highlight when content is present).
- Content area uses `svelte/transition` `slide` for smooth expand/collapse.

### ImageChipBar

- `images` is bindable; `onremove` overrides the default removal from the array; `onopen` is called when a thumbnail is opened.
- Renders nothing when `images` is empty.
- Chip label is `Image #{image.number ?? position + 1}`: the optional `number` of `ConversationImage` wins, the 1-based position is the fallback.
- Each chip is an `AttachmentChip` with a thumbnail from the base64 data URI (120 px, or 64 px with `variant="small"`). Delete removes the image from the bound array unless `onremove` is given.
- Import `ConversationImage` from `$lib/types/conversation`.

### ContextEditor

Lives in `shared/widgets/ContextEditor.svelte` (it calls `openImagePreview` from `$lib/services/windowPreviews` for `ImageChipBar`'s `onopen`), not here. Parents control persistence.

### CommandPalette

- `inputRef` is bindable so the consumer can call `.focus()`.
- Pure visual shell: shared frame (`var(--surface-floating-modal)` with `--surface-floating-modal-border`), input row, scrollable body, footer. Consumers render the list and footer keys via snippets.
- `variant="overlay"` is for in-window overlays (e.g. Cmd+K inside ConversationDialog) — adds the `--surface-scrim` backdrop with fade-in.
- `variant="window"` is for separate Tauri windows with transparent body (e.g. Cmd+P provider switcher) — keeps the scrim transparent so the host window itself acts as the backdrop. On Linux this variant uses a lighter shadow (`--shadow-lg-linux`) because GTK transparent windows render heavy shadows poorly; the overlay variant keeps `--shadow-lg` on every platform.
- Provides utility classes for consumers: `.palette-item`, `.palette-item.highlight`, `.palette-item-icon`, `.palette-item-name`, `.palette-empty`, `.palette-divider`, plus `.palette-footer kbd` styling.

### MarkdownRenderer

- `onopen` and `onsavesvg` are optional callbacks (open a link, save a Mermaid SVG).
- Uses `marked` + `highlight.js` via `$lib/utils/markdown.ts` to render markdown with syntax-highlighted code blocks.
- `renderMarkdown` always highlights code blocks (`highlight.js`, auto-detecting the language when none is given); the renderer's `isStreaming` only paces how much of `content` is shown, not highlighting.
- Code copy: each code block gets a "Copy" button with a `data-copy-index` attribute. Click handler uses event delegation on the container div to copy the raw code block content to clipboard.

