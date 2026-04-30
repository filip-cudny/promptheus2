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
      />
      {#if headerExtras}{@render headerExtras()}{/if}
    </div>
    <div
      class="palette-body"
      role="listbox"
      style="max-height: {bodyMaxHeight}"
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
    padding: 0;
    cursor: default;
    z-index: 1000;
  }

  .palette-scrim.scrim-overlay {
    background: rgba(0, 0, 0, 0.5);
    animation: palette-scrim-enter 140ms ease-out both;
  }

  .palette-modal {
    position: fixed;
    top: 80px;
    left: 50%;
    transform: translateX(-50%);
    width: min(640px, 86%);
    z-index: 1001;
    background: #252525;
    border: 1px solid rgba(255, 255, 255, 0.08);
    border-radius: 8px;
    box-shadow: 0 20px 60px rgba(0, 0, 0, 0.5);
    overflow: hidden;
    display: flex;
    flex-direction: column;
    color: #e0e0e0;
    font-size: 13px;
    animation: palette-modal-enter 140ms ease-out both;
  }

  :global([data-platform="linux"]) .palette-modal.window-variant {
    box-shadow: 0 6px 18px rgba(0, 0, 0, 0.22);
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
    border-bottom: 1px solid rgba(255, 255, 255, 0.06);
  }

  .palette-input {
    flex: 1;
    appearance: none;
    border: 0;
    background: transparent;
    color: #fff;
    font: inherit;
    font-size: 14px;
    padding: 12px 14px;
    outline: none;
  }

  .palette-input::placeholder {
    color: rgba(255, 255, 255, 0.35);
  }

  .palette-body {
    display: flex;
    flex-direction: column;
    overflow-y: auto;
    padding: 4px 0;
  }

  .palette-footer {
    border-top: 1px solid rgba(255, 255, 255, 0.06);
    padding: 6px 14px;
    display: flex;
    gap: 12px;
    color: rgba(255, 255, 255, 0.4);
    font-size: 11px;
  }

  :global(.palette-footer kbd) {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    background: rgba(255, 255, 255, 0.08);
    border: 1px solid rgba(255, 255, 255, 0.1);
    border-radius: 3px;
    padding: 1px 5px;
    font-family: inherit;
    font-size: 10px;
    line-height: 1;
    color: rgba(255, 255, 255, 0.7);
    margin-right: 4px;
    vertical-align: middle;
  }

  :global(.palette-item) {
    appearance: none;
    border: 0;
    background: transparent;
    color: rgba(255, 255, 255, 0.85);
    font: inherit;
    text-align: left;
    padding: 8px 14px;
    cursor: pointer;
    display: flex;
    align-items: center;
    gap: 12px;
  }

  :global(.palette-item.highlight) {
    background: rgba(255, 255, 255, 0.08);
  }

  :global(.palette-item-icon) {
    width: 16px;
    height: 16px;
    flex-shrink: 0;
    display: inline-flex;
    align-items: center;
    justify-content: center;
    color: rgba(255, 255, 255, 0.85);
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
    font-size: 13px;
  }

  :global(.palette-empty) {
    color: rgba(255, 255, 255, 0.4);
    padding: 16px;
    text-align: center;
    font-size: 12px;
  }

  :global(.palette-divider) {
    height: 1px;
    background: rgba(255, 255, 255, 0.06);
    margin: 4px 0;
  }
</style>
