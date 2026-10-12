<script lang="ts">
  import { MessageSquare } from "lucide-svelte";
  import { ICON_SIZE } from "$lib/constants/ui";

  type Props = {
    disabled: boolean;
    rowEl?: HTMLElement;
    onclick: (e: MouseEvent) => void;
    oncontextmenu: (e: MouseEvent) => void;
    onhover: () => void;
  };

  let {
    disabled,
    rowEl = $bindable(),
    onclick,
    oncontextmenu,
    onhover,
  }: Props = $props();
</script>

<!-- svelte-ignore a11y_no_static_element_interactions -->
<div
  class="chat-row"
  role="menuitem"
  bind:this={rowEl}
  onmouseenter={onhover}
  {oncontextmenu}
>
  <button class="chat-button" class:disabled {onclick}>
    <MessageSquare size={ICON_SIZE.md} />
    <span>Chat</span>
  </button>
</div>

<style>
  .chat-row {
    display: flex;
    align-items: center;
  }

  .chat-row:hover {
    background: var(--surface-overlay);
  }

  .chat-row:active {
    background: rgba(255, 255, 255, 0.15);
  }

  .chat-button {
    display: flex;
    align-items: center;
    gap: var(--space-4);
    flex: 1;
    min-width: 0;
    padding: var(--space-3) var(--space-6);
    border: none;
    background: transparent;
    color: var(--text-primary);
    font: inherit;
    text-align: left;
    cursor: pointer;
    box-sizing: border-box;
    outline: none;
  }

  .chat-button.disabled {
    color: var(--text-disabled);
    cursor: default;
  }
</style>
