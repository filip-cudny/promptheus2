<script lang="ts">
  import { KeyRound, MessageCircleQuestion, PencilLine, Search, Sparkles } from "lucide-svelte";
  import { ICON_SIZE } from "$lib/constants/ui";
  import { getSettingsAgent } from "$lib/stores/settingsAgent.svelte";

  const agent = getSettingsAgent();

  const lines = [
    { icon: Search, text: "Find where an option is" },
    { icon: MessageCircleQuestion, text: "Ask how something is set" },
    { icon: PencilLine, text: "Change it: you approve a card first" },
    { icon: KeyRound, text: "Secrets: shown to you masked, never to the agent" },
  ];

  const chips = [
    "Where is the context menu shortcut?",
    "Which model does Quick actions use?",
    "Where do I set the speech language?",
  ];
</script>

<div class="island">
  <div class="title"><Sparkles size={ICON_SIZE.md} />Ask instead of searching</div>
  <ul class="lines">
    {#each lines as line (line.text)}
      <li><line.icon size={ICON_SIZE.sm} />{line.text}</li>
    {/each}
  </ul>
  <div class="chips">
    {#each chips as chip (chip)}
      <button class="chip" onclick={() => agent.fillDraft(chip)}>{chip}</button>
    {/each}
  </div>
</div>

<style>
  .island {
    border: 1px solid var(--border-faint);
    border-radius: var(--radius-xl);
    padding: var(--space-6) var(--space-7);
  }

  .title {
    display: flex;
    align-items: center;
    gap: var(--space-4);
    margin-bottom: var(--space-4);
    font-weight: 500;
    color: var(--text-secondary);
  }

  .title :global(svg) {
    color: var(--accent);
  }

  .lines {
    list-style: none;
    margin: 0 0 var(--space-6);
    padding: 0;
    display: flex;
    flex-direction: column;
    gap: var(--space-2);
    font-size: var(--font-size-md);
    color: var(--text-muted);
  }

  .lines li {
    display: flex;
    align-items: center;
    gap: var(--space-4);
  }

  .lines :global(svg) {
    flex-shrink: 0;
    color: var(--text-faint);
  }

  .chips {
    display: flex;
    flex-direction: column;
    gap: var(--space-2);
  }

  .chip {
    text-align: left;
    padding: var(--space-2) var(--space-4);
    background: transparent;
    border: 1px solid var(--border-faint);
    border-radius: var(--radius-lg);
    color: var(--text-muted);
    font: inherit;
    font-size: var(--font-size-md);
    cursor: pointer;
  }

  .chip:hover {
    border-color: var(--accent-border);
    color: var(--text-primary);
    background: var(--accent-bg-soft);
  }
</style>
