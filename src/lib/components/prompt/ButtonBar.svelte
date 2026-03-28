<script lang="ts">
  import type { createConversationStore } from "$lib/stores/conversation.svelte";

  let {
    store,
    onSendAndCopy,
  }: {
    store: ReturnType<typeof createConversationStore>;
    onSendAndCopy: () => void;
  } = $props();

  function handleSendShow() {
    if (store.isRegenerateMode) {
      const path = store.tree.current_path;
      if (path.length > 0) {
        store.regenerate(path[path.length - 1]);
      }
    } else {
      store.sendMessage();
    }
  }

  const primaryLabel = $derived.by(() => {
    if (store.isExecuting) return "Stop";
    if (store.isRegenerateMode) return "Regenerate";
    return "Send";
  });
</script>

<div class="button-bar">
  <div class="bar-left">
    <button class="icon-btn" onclick={() => store.addTab()} title="New tab">+</button>
  </div>

  <div class="bar-center">
    <!-- Tab bar placeholder — implemented in task 9 -->
  </div>

  <div class="bar-right">
    <button
      class="btn btn-secondary"
      onclick={onSendAndCopy}
      disabled={!store.canSend || store.isExecuting}
      title="Ctrl+Enter"
    >
      Send & Copy
    </button>

    {#if store.isExecuting}
      <button class="btn btn-danger" onclick={() => store.stopExecution()}>
        Stop
      </button>
    {:else}
      <button
        class="btn btn-primary"
        onclick={handleSendShow}
        disabled={!store.canSend && !store.isRegenerateMode}
        title="Enter"
      >
        {primaryLabel}
      </button>
    {/if}
  </div>
</div>

<style>
  .button-bar {
    flex-shrink: 0;
    display: flex;
    align-items: center;
    gap: 8px;
    padding: 8px 12px;
    border-top: 1px solid rgba(255, 255, 255, 0.08);
    background: rgba(255, 255, 255, 0.02);
  }

  .bar-left {
    flex-shrink: 0;
  }

  .bar-center {
    flex: 1;
    min-width: 0;
  }

  .bar-right {
    flex-shrink: 0;
    display: flex;
    gap: 6px;
  }

  .icon-btn {
    width: 28px;
    height: 28px;
    border-radius: 4px;
    border: 1px solid rgba(255, 255, 255, 0.15);
    background: rgba(255, 255, 255, 0.06);
    color: #e0e0e0;
    font-size: 16px;
    cursor: pointer;
    display: flex;
    align-items: center;
    justify-content: center;
    padding: 0;
  }

  .icon-btn:hover {
    background: rgba(255, 255, 255, 0.12);
  }

  .btn {
    padding: 6px 14px;
    border-radius: 4px;
    border: none;
    font-size: 12px;
    font-weight: 600;
    cursor: pointer;
    white-space: nowrap;
  }

  .btn:disabled {
    opacity: 0.4;
    cursor: not-allowed;
  }

  .btn-primary {
    background: rgba(100, 160, 255, 0.8);
    color: #fff;
  }

  .btn-primary:hover:not(:disabled) {
    background: rgba(100, 160, 255, 1);
  }

  .btn-secondary {
    background: rgba(255, 255, 255, 0.1);
    color: #e0e0e0;
  }

  .btn-secondary:hover:not(:disabled) {
    background: rgba(255, 255, 255, 0.18);
  }

  .btn-danger {
    background: rgba(220, 60, 60, 0.8);
    color: #fff;
  }

  .btn-danger:hover {
    background: rgba(220, 60, 60, 1);
  }
</style>
