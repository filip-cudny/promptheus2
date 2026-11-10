<script lang="ts">
  import { openUrl } from "@tauri-apps/plugin-opener";
  import MarkdownRenderer from "$lib/components/shared/ui/MarkdownRenderer.svelte";
  import { getSettingsNavigation } from "$lib/stores/settingsNavigation.svelte";
  import { getSettingsAgent } from "$lib/stores/settingsAgent.svelte";
  import ToolCallGroup from "$lib/components/features/conversation-dialog/tool-call/ToolCallGroup.svelte";
  import ThinkingBlock from "$lib/components/shared/ui/ThinkingBlock.svelte";
  import ProcessingIndicator from "$lib/components/features/conversation-dialog/components/ProcessingIndicator.svelte";
  import SecretField from "../SecretField.svelte";
  import ProposalCard from "./ProposalCard.svelte";
  import { parseSettingLink, showSettingLinks } from "$lib/utils/settingLinks";
  import { showsProcessing, thinkingText } from "$lib/utils/agentReply";
  import { parseContentSegments, toRenderBlocks } from "$lib/utils/toolCallMarkers";
  import type { ToolCall } from "$lib/types/ai";

  let { docked }: { docked: boolean } = $props();

  const agent = getSettingsAgent();
  const navigation = getSettingsNavigation();

  async function open(url: string) {
    const key = parseSettingLink(url);
    if (!key) {
      openUrl(url);
      return;
    }
    if (!docked) await agent.hidePanel("show-link");
    navigation.navigateTo(key);
  }

  function groupCalls(ids: string[], calls: ToolCall[]): ToolCall[] {
    return ids
      .map((id) => calls.find((call) => call.tool_call_id === id))
      .filter((call): call is ToolCall => call != null);
  }
</script>

<div class="messages">
  <div class="column">
    {#each agent.messages as message, i (message.id)}
      {#if message.role === "user"}
        <div class="user">{message.text}</div>
      {:else}
        {@const reasoning = thinkingText(message)}
        <div class="assistant">
          {#if reasoning || message.isThinking}
            <ThinkingBlock
              thinkingContent={reasoning}
              isThinkingActive={message.isThinking}
              isStreaming={agent.running && i === agent.messages.length - 1}
            />
          {/if}
          {#each toRenderBlocks(parseContentSegments(message.text)) as block}
            {#if block.kind === "text"}
              <MarkdownRenderer
                content={showSettingLinks(block.text)}
                isStreaming={agent.running && i === agent.messages.length - 1}
                onopen={open}
              />
            {:else}
              {@const calls = groupCalls(block.toolCallIds, message.toolCalls)}
              {#if calls.length > 0}
                <ToolCallGroup toolCalls={calls} isStreaming={agent.running && i === agent.messages.length - 1} />
              {/if}
            {/if}
          {/each}
          {#each message.secretViews as name, view (view)}
            <SecretField {name} readonly />
          {/each}
          {#if i === agent.messages.length - 1 && showsProcessing(message, agent.running)}
            <ProcessingIndicator />
          {/if}
        </div>
      {/if}
    {/each}
    {#if agent.proposal}
      {#key agent.proposal.proposal_id}
        <ProposalCard proposal={agent.proposal} />
      {/key}
    {/if}
    {#if agent.error}
      <div class="error">{agent.error}</div>
    {/if}
  </div>
</div>

<style>
  .messages {
    flex: 1;
    min-height: 0;
    overflow-y: auto;
    padding: var(--space-6);
  }

  .column {
    display: flex;
    flex-direction: column;
    gap: var(--space-6);
    width: 100%;
    max-width: 760px;
    margin: 0 auto;
  }

  .user {
    align-self: flex-end;
    max-width: 85%;
    padding: 7px 11px;
    background: var(--accent-bg-soft);
    border-radius: 10px 10px 2px 10px;
    white-space: pre-wrap;
  }

  .assistant {
    line-height: 1.55;
  }

  .error {
    color: var(--danger, #e5484d);
    font-size: var(--font-size-sm);
  }

  .assistant :global(a[href^="setting:"]) {
    padding: 0 var(--space-3);
    border: 1px solid var(--border-faint);
    border-radius: var(--radius-md);
    background: var(--surface-overlay-faint);
    color: var(--text-primary);
    text-decoration: none;
    font-size: var(--font-size-xs);
  }
</style>
