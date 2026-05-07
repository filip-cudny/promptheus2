<script lang="ts">
  import type { Snippet } from "svelte";

  let {
    open,
    onClose,
    query = $bindable(""),
    placeholder,
    variant,
    bodyMaxHeight = "360px",
    inputRef = $bindable(),
    headerExtras,
    body,
    footer,
  }: {
    open: boolean;
    onClose: () => void;
    query?: string;
    placeholder: string;
    variant: "overlay" | "window";
    bodyMaxHeight?: string;
    inputRef?: HTMLInputElement | undefined;
    headerExtras?: Snippet;
    body: Snippet;
    footer?: Snippet;
  } = $props();

  let bodyEl: HTMLDivElement | undefined = $state();
  let pinnedBodyH = $state<string | null>(null);

  $effect(() => {
    if (!open) pinnedBodyH = null;
  });

  function captureBodyHeight() {
    if (pinnedBodyH !== null || !bodyEl) return;
    const h = bodyEl.offsetHeight;
    if (h < 1) return;
    pinnedBodyH = `${h}px`;
  }
</script>

{#if open}
  <button
    type="button"
    aria-label="Close palette"
    class="palette-scrim"
    class:scrim-overlay={variant === "overlay"}
    onclick={onClose}
  ></button>
  <div
    class="palette-modal"
    class:window-variant={variant === "window"}
    role="dialog"
    aria-modal="true"
  >
    <div class="palette-header">
      <input
        bind:this={inputRef}
        bind:value={query}
        class="palette-input"
        type="text"
        {placeholder}
        autocomplete="off"
        spellcheck="false"
        onbeforeinput={captureBodyHeight}
      />
      {#if headerExtras}{@render headerExtras()}{/if}
    </div>
    <div
      bind:this={bodyEl}
      class="palette-body"
      role="listbox"
      data-pinned={pinnedBodyH ? "" : undefined}
      style="--palette-body-h: {bodyMaxHeight}; {pinnedBodyH ? `--palette-body-pin: ${pinnedBodyH};` : ''}"
    >
      {@render body()}
    </div>
    {#if footer}
      <div class="palette-footer">{@render footer()}</div>
    {/if}
  </div>
{/if}

<style>
  .palette-scrim {
    position: fixed;
    inset: 0;
    background: transparent;
    border: 0;
    padding: var(--space-0);
    cursor: default;
    z-index: var(--z-overlay);
  }

  .palette-scrim.scrim-overlay {
    background: var(--surface-scrim);
    animation: palette-scrim-enter 140ms ease-out both;
  }

  .palette-modal {
    position: fixed;
    top: 80px;
    left: 50%;
    transform: translateX(-50%);
    width: min(640px, 86%);
    z-index: var(--z-modal);
    background: var(--surface-floating-modal);
    border: 1px solid var(--surface-floating-modal-border);
    border-radius: var(--radius-xl);
    box-shadow: var(--shadow-lg);
    overflow: hidden;
    display: flex;
    flex-direction: column;
    color: var(--text-primary);
    font-size: var(--font-size-base);
    animation: palette-modal-enter 140ms ease-out both;
  }

  :global([data-platform="linux"]) .palette-modal.window-variant {
    box-shadow: var(--shadow-lg-linux);
  }

  @keyframes palette-scrim-enter {
    from { opacity: 0; }
    to { opacity: 1; }
  }

  @keyframes palette-modal-enter {
    from { opacity: 0; }
    to { opacity: 1; }
  }

  .palette-header {
    display: flex;
    align-items: center;
    border-bottom: 1px solid var(--border-faint);
  }

  .palette-input {
    flex: 1;
    appearance: none;
    border: 0;
    background: transparent;
    color: var(--text-primary);
    font: inherit;
    font-size: var(--font-size-lg);
    padding: var(--space-6) var(--space-7);
    outline: none;
  }

  .palette-input::placeholder {
    color: var(--text-disabled);
  }

  .palette-body {
    display: flex;
    flex-direction: column;
    overflow-y: auto;
    padding: var(--space-2) var(--space-0);
    max-height: var(--palette-body-h);
  }

  :global([data-platform="linux"]) .palette-modal.window-variant .palette-body[data-pinned] {
    min-height: var(--palette-body-pin);
    max-height: var(--palette-body-pin);
  }

  .palette-footer {
    border-top: 1px solid var(--border-faint);
    padding: var(--space-3) var(--space-7);
    display: flex;
    gap: var(--space-6);
    color: var(--text-disabled);
    font-size: var(--font-size-sm);
  }

  :global(.palette-item) {
    appearance: none;
    border: 0;
    background: transparent;
    color: var(--text-primary);
    font: inherit;
    text-align: left;
    padding: var(--space-4) var(--space-7);
    cursor: pointer;
    display: flex;
    align-items: center;
    gap: var(--space-6);
  }

  :global(.palette-item.highlight) {
    background: var(--surface-overlay);
  }

  :global(.palette-item-icon) {
    width: 16px;
    height: 16px;
    flex-shrink: 0;
    display: inline-flex;
    align-items: center;
    justify-content: center;
    color: var(--text-primary);
  }

  :global(.palette-item-icon svg) {
    width: 100%;
    height: 100%;
    display: block;
  }

  :global(.palette-item-icon img) {
    width: 100%;
    height: 100%;
    display: block;
    object-fit: contain;
  }

  :global(.palette-item-name) {
    flex: 1;
    font-size: var(--font-size-base);
  }

  :global(.palette-empty) {
    color: var(--text-disabled);
    padding: var(--space-8);
    text-align: center;
    font-size: var(--font-size-md);
  }

  :global(.palette-divider) {
    height: 1px;
    background: var(--surface-overlay-faint);
    margin: var(--space-2) var(--space-0);
  }
</style>
