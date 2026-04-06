<script lang="ts">
  import { Loader2 } from "lucide-svelte";
  import { ICON_SIZE } from "$lib/constants/ui";
  import type { createConversationStore } from "$lib/stores/conversation.svelte";
  import type { MessagePair } from "$lib/types/conversation";
  import UserBubble from "./UserBubble.svelte";
  import AssistantBubble from "./AssistantBubble.svelte";

  const PAGE_SIZE = 20;

  let {
    store,
  }: {
    store: ReturnType<typeof createConversationStore>;
  } = $props();

  let container: HTMLDivElement | undefined = $state();
  let userScrolledUp = $state(false);
  let visibleCount = $state(PAGE_SIZE);

  let allPairs = $derived(store.messagePairs);
  let hasMore = $derived(visibleCount < allPairs.length);
  let visiblePairs = $derived(
    hasMore ? allPairs.slice(allPairs.length - visibleCount) : allPairs,
  );

  $effect(() => {
    store.activeTabId;
    visibleCount = PAGE_SIZE;
  });

  function handleScroll() {
    if (!container) return;
    const threshold = 50;
    const distanceFromBottom =
      container.scrollHeight - container.scrollTop - container.clientHeight;
    userScrolledUp = distanceFromBottom > threshold;

    if (container.scrollTop < 100 && hasMore) {
      loadMore();
    }
  }

  function loadMore() {
    if (!container || !hasMore) return;
    const prevHeight = container.scrollHeight;
    visibleCount = Math.min(visibleCount + PAGE_SIZE, allPairs.length);
    requestAnimationFrame(() => {
      if (container) {
        container.scrollTop += container.scrollHeight - prevHeight;
      }
    });
  }

  function isLastAssistant(pair: MessagePair): boolean {
    const pairs = allPairs;
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

  $effect(() => {
    if (allPairs.length > visibleCount) {
      visibleCount = Math.max(visibleCount, PAGE_SIZE);
    }
  });
</script>

<div class="conversation-area" bind:this={container} onscroll={handleScroll}>
  {#if allPairs.length === 0}
    <div class="empty-state">Send a message to start the conversation.</div>
  {:else}
    {#if hasMore}
      <div class="load-more-zone">
        <Loader2 size={ICON_SIZE.sm} class="spin" />
        <span>Scroll up for older messages</span>
      </div>
    {/if}
    {#each visiblePairs as pair (pair.user.node_id)}
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
          thinkingContent={streaming ? store.streamedThinking : (assistant.thinking ?? "")}
          isThinkingActive={streaming && store.isThinking}
          branchInfo={store.getBranchInfo(assistant.node_id)}
          activeToolCalls={streaming ? store.activeToolCalls : []}
          onRegenerate={() => store.regenerate(assistant.node_id)}
          onBranchPrev={() => store.switchBranch(assistant.node_id, -1)}
          onBranchNext={() => store.switchBranch(assistant.node_id, 1)}
          onContentChange={(content) => store.updateNodeContent(assistant.node_id, content)}
          onDelete={() => {}}
          onToolCallApprove={(id) => store.approveToolCall(id)}
          onToolCallReject={(id) => store.rejectToolCall(id)}
          onToolCallRetry={(id) => store.retryToolCall(id)}
        />
      {/if}
    {/each}
  {/if}
</div>

<style>
  .conversation-area {
    flex: 1;
    overflow-y: auto;
    padding: 40px 16px 16px;
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

  .load-more-zone {
    display: flex;
    align-items: center;
    justify-content: center;
    gap: 6px;
    padding: 8px;
    color: rgba(255, 255, 255, 0.3);
    font-size: 11px;
  }

  .load-more-zone :global(.spin) {
    animation: spin 1s linear infinite;
  }

  @keyframes spin {
    from { transform: rotate(0deg); }
    to { transform: rotate(360deg); }
  }
</style>
