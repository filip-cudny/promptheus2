<script lang="ts">
  import { openUrl } from "@tauri-apps/plugin-opener";
  import MarkdownRenderer from "$lib/components/shared/ui/MarkdownRenderer.svelte";
  import { getSettingsNavigation } from "$lib/stores/settingsNavigation.svelte";
  import { getSettingsAgent } from "$lib/stores/settingsAgent.svelte";
  import SecretField from "../SecretField.svelte";
  import ProposalCard from "./ProposalCard.svelte";
  import { parseSettingLink, showSettingLinks } from "$lib/utils/settingLinks";

  const agent = getSettingsAgent();
  const navigation = getSettingsNavigation();

  function open(url: string) {
    const key = parseSettingLink(url);
    if (key) navigation.navigateTo(key);
    else openUrl(url);
  }
</script>

<div class="messages">
  {#each agent.messages as message, i (message.id)}
    {#if message.role === "user"}
      <div class="user">{message.text}</div>
    {:else}
      <div class="assistant">
        {#if message.toolCalls.length > 0}
          <div class="chips">
            {#each message.toolCalls as call (call.id)}
              <span class="chip" data-status={call.status}>{call.name}</span>
            {/each}
          </div>
        {/if}
        {#if message.text}
          <MarkdownRenderer
            content={showSettingLinks(message.text)}
            isStreaming={agent.running && i === agent.messages.length - 1}
            onopen={open}
          />
        {/if}
      </div>
    {/if}
  {/each}
  {#each agent.secretRequests as request (request.request_id)}
    <SecretField
      name={request.name}
      purpose={request.purpose}
      exists={request.exists}
      onclose={(outcome) => agent.resolveSecret(request.request_id, outcome)}
    />
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

<style>
  .messages {
    flex: 1;
    min-height: 0;
    overflow-y: auto;
    display: flex;
    flex-direction: column;
    gap: var(--space-6);
    padding: var(--space-6);
  }

  .user {
    align-self: flex-end;
    max-width: 90%;
    padding: var(--space-3) var(--space-5);
    background: var(--accent-bg-soft);
    border-radius: var(--radius-md);
    white-space: pre-wrap;
  }

  .chips {
    display: flex;
    flex-wrap: wrap;
    gap: var(--space-2);
    margin-bottom: var(--space-3);
  }

  .chip {
    padding: 1px var(--space-4);
    font-size: var(--font-size-xs);
    color: var(--text-secondary);
    background: var(--surface-overlay-faint);
    border-radius: var(--radius-md);
  }

  .chip[data-status="failed"] {
    color: var(--danger, #e5484d);
  }

  .chip[data-status="in_progress"] {
    opacity: 0.7;
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
