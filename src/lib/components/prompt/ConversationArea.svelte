<script lang="ts">
  import type { createConversationStore } from "$lib/stores/conversation.svelte";

  let {
    store,
  }: {
    store: ReturnType<typeof createConversationStore>;
  } = $props();

  let container: HTMLDivElement | undefined = $state();
  let userScrolledUp = $state(false);

  function handleScroll() {
    if (!container) return;
    const threshold = 50;
    const distanceFromBottom =
      container.scrollHeight - container.scrollTop - container.clientHeight;
    userScrolledUp = distanceFromBottom > threshold;
  }

  $effect(() => {
    store.messagePairs;
    store.streamedContent;
    if (!userScrolledUp && container) {
      requestAnimationFrame(() => {
        container!.scrollTop = container!.scrollHeight;
      });
    }
  });
</script>

<div class="conversation-area" bind:this={container} onscroll={handleScroll}>
  {#if store.messagePairs.length === 0}
    <div class="empty-state">Send a message to start the conversation.</div>
  {/if}
</div>

<style>
  .conversation-area {
    flex: 1;
    overflow-y: auto;
    padding: 12px 16px;
    display: flex;
    flex-direction: column;
    gap: 8px;
  }

  .empty-state {
    flex: 1;
    display: flex;
    align-items: center;
    justify-content: center;
    color: rgba(255, 255, 255, 0.3);
    font-size: 14px;
  }
</style>
