<script lang="ts">
  import Button from "$lib/components/shared/ui/Button.svelte";
  import { getSettingsAgent } from "$lib/stores/settingsAgent.svelte";
  import { getSettingsNavigation } from "$lib/stores/settingsNavigation.svelte";
  import type { ProposalData, ProposalEffect } from "$lib/services/settingsAgent";
  import ProposalRow from "./ProposalRow.svelte";

  const agent = getSettingsAgent();
  const navigation = getSettingsNavigation();

  let { proposal }: { proposal: ProposalData } = $props();

  let unchecked = $state<Record<number, boolean>>({});

  const outcome = $derived(agent.proposalOutcome);
  const applied = $derived(outcome?.kind === "applied");
  const hasStale = $derived(proposal.rows.some((r) => r.stale));
  const anySelected = $derived(proposal.rows.some((r) => !unchecked[r.row]));

  function selection(): boolean[] {
    const size = Math.max(...proposal.rows.map((r) => r.row)) + 1;
    return Array.from({ length: size }, (_, i) => proposal.rows.some((r) => r.row === i) && !unchecked[i]);
  }

  function show(effect: ProposalEffect) {
    if (effect.target.type === "setting") navigation.navigateTo(effect.target.path);
    else if (effect.target.path.startsWith("skills/")) navigation.activeSection = "skills";
    else navigation.activeSection = "prompt_base";
  }
</script>

<div class="card">
  {#each proposal.rows as row (row.row)}
    <ProposalRow
      {row}
      bind:checked={() => !unchecked[row.row], (v) => (unchecked[row.row] = !v)}
      disabled={applied}
      onshow={show}
      ontest={() => agent.testRow(row.row)}
    />
  {/each}
  {#if outcome?.kind === "blocked"}
    <div class="blocked">{outcome.message}</div>
  {/if}
  <div class="actions">
    {#if applied}
      <span class="applied">Applied</span>
    {:else}
      {#if hasStale}
        <Button variant="ghost" onclick={() => agent.refreshProposal()}>Refresh</Button>
      {/if}
      <Button variant="ghost" onclick={() => agent.rejectProposal()}>Reject</Button>
      <Button
        variant="primary"
        disabled={hasStale || !anySelected || agent.applying}
        onclick={() => agent.applyProposal(selection())}>Apply</Button
      >
    {/if}
  </div>
</div>

<style>
  .card {
    display: flex;
    flex-direction: column;
    padding: var(--space-3) var(--space-5);
    background: var(--surface-raised);
    border: 1px solid var(--border-default);
    border-radius: var(--radius-md);
  }

  .blocked {
    color: var(--danger);
    font-size: var(--font-size-sm);
    padding: var(--space-2) 0;
  }

  .actions {
    display: flex;
    justify-content: flex-end;
    align-items: center;
    gap: var(--space-2);
    padding-top: var(--space-3);
    border-top: 1px solid var(--border-faint);
  }

  .applied {
    color: var(--success);
    font-weight: var(--font-weight-semibold);
  }
</style>
