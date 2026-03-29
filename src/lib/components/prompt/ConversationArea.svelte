<script lang="ts">
  import type { createConversationStore } from "$lib/stores/conversation.svelte";
  import type { MessagePair } from "$lib/types/conversation";
  import UserBubble from "./UserBubble.svelte";
  import AssistantBubble from "./AssistantBubble.svelte";

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

  function isLastAssistant(pair: MessagePair): boolean {
    const pairs = store.messagePairs;
    return pairs[pairs.length - 1] === pair;
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
  {:else}
    {#each store.messagePairs as pair (pair.user.node_id)}
      <UserBubble
        node={pair.user}
        messageNumber={pair.message_number}
        showDelete={false}
        onContentChange={(content) => store.updateNodeContent(pair.user.node_id, content)}
        onDelete={() => {}}
        onRegenerate={() => { if (pair.assistant) store.regenerate(pair.assistant.node_id); }}
      />
      {#if pair.assistant}
        {@const assistant = pair.assistant}
        {@const streaming = store.isStreaming && isLastAssistant(pair)}
        <AssistantBubble
          node={assistant}
          displayContent={streaming ? store.streamedContent : assistant.content}
          outputNumber={pair.message_number}
          showDelete={false}
          isStreaming={streaming}
          branchInfo={store.getBranchInfo(assistant.node_id)}
          onRegenerate={() => store.regenerate(assistant.node_id)}
          onBranchPrev={() => store.switchBranch(assistant.node_id, -1)}
          onBranchNext={() => store.switchBranch(assistant.node_id, 1)}
          onContentChange={(content) => store.updateNodeContent(assistant.node_id, content)}
          onDelete={() => {}}
        />
      {/if}
    {/each}
  {/if}
</div>

<style>
  .conversation-area {
    flex: 1;
    overflow-y: auto;
    padding: 40px 16px 12px;
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
