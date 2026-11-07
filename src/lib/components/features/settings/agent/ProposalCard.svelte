<script lang="ts">
  import { onDestroy } from "svelte";
  import { Undo2 } from "lucide-svelte";
  import { ICON_SIZE } from "$lib/constants/ui";
  import Button from "$lib/components/shared/ui/Button.svelte";
  import { getSettingsAgent } from "$lib/stores/settingsAgent.svelte";
  import { getSettingsNavigation } from "$lib/stores/settingsNavigation.svelte";
  import type { ProposalData, ProposalEffect, ProposalRowData } from "$lib/services/settingsAgent";
  import ProposalRow from "./ProposalRow.svelte";
  import UndoConflicts from "../UndoConflicts.svelte";

  const agent = getSettingsAgent();
  const navigation = getSettingsNavigation();

  let { proposal }: { proposal: ProposalData } = $props();

  const RETEST_DEBOUNCE_MS = 500;

  let unchecked = $state<Record<number, boolean>>({});
  let values = $state<Record<string, string>>({});
  const retestTimers: Record<string, ReturnType<typeof setTimeout>> = {};

  onDestroy(() => Object.values(retestTimers).forEach(clearTimeout));

  const outcome = $derived(agent.proposalOutcome);
  const done = $derived(outcome?.kind === "applied" || outcome?.kind === "undone");
  const hasStale = $derived(proposal.rows.some((r) => r.stale));
  const anySelected = $derived(proposal.rows.some((r) => !unchecked[r.row]));

  function selection(): boolean[] {
    const size = Math.max(...proposal.rows.map((r) => r.row)) + 1;
    return Array.from({ length: size }, (_, i) => proposal.rows.some((r) => r.row === i) && !unchecked[i]);
  }

  function stagedValues(selectedOnly: boolean): Record<string, string> {
    const staged: Record<string, string> = {};
    for (const r of proposal.rows) {
      const name = r.env_value?.name;
      if (name && values[name] && (!selectedOnly || !unchecked[r.row])) staged[name] = values[name];
    }
    return staged;
  }

  function missingValues(row: ProposalRowData): string[] {
    return row.waits_for.filter((name) => !values[name]);
  }

  function retestWaiting(name: string) {
    for (const r of proposal.rows) {
      if (r.waits_for.includes(name) && r.new_destination === null && missingValues(r).length === 0) {
        void agent.testRow(r.row, stagedValues(false));
      }
    }
  }

  function scheduleRetest(name: string) {
    clearTimeout(retestTimers[name]);
    retestTimers[name] = setTimeout(() => flushRetest(name), RETEST_DEBOUNCE_MS);
  }

  function flushRetest(name: string) {
    if (!(name in retestTimers)) return;
    clearTimeout(retestTimers[name]);
    delete retestTimers[name];
    retestWaiting(name);
  }

  function show(effect: ProposalEffect) {
    const { target } = effect;
    if (target.type === "setting") navigation.navigateTo(target.path);
    else if (target.type === "file") {
      if (target.path.startsWith("skills/")) navigation.navigateTo(`skills[${target.path.split("/")[1]}]`);
      else navigation.activeSection = "prompt_base";
    }
  }
</script>

<div class="card">
  {#each proposal.rows as row (row.row)}
    {@const name = row.env_value?.name ?? ""}
    <ProposalRow
      {row}
      bind:checked={() => !unchecked[row.row], (v) => (unchecked[row.row] = !v)}
      bind:value={() => values[name] ?? "", (v) => (values[name] = v)}
      missing={missingValues(row)}
      disabled={done}
      onshow={show}
      ontest={() => agent.testRow(row.row, stagedValues(false))}
      onvalueinput={() => scheduleRetest(name)}
      onvalueblur={() => flushRetest(name)}
    />
  {/each}
  {#if outcome && "message" in outcome && outcome.message}
    <div class="blocked">{outcome.message}</div>
  {/if}
  {#if outcome?.kind === "applied" && outcome.conflicts}
    <UndoConflicts
      conflicts={outcome.conflicts}
      disabled={agent.undoing}
      oncancel={() => agent.dismissUndoConflicts()}
      onforce={() => agent.undoProposal(true)}
    />
  {/if}
  <div class="actions">
    {#if outcome?.kind === "applied"}
      <span class="applied">Applied</span>
      <Button variant="ghost" disabled={agent.undoing} onclick={() => agent.undoProposal(false)}>
        <Undo2 size={ICON_SIZE.sm} /> Undo
      </Button>
    {:else if outcome?.kind === "undone"}
      <span class="undone">Undone</span>
    {:else}
      {#if hasStale}
        <Button variant="ghost" onclick={() => agent.refreshProposal()}>Refresh</Button>
      {/if}
      <Button variant="ghost" onclick={() => agent.rejectProposal()}>Reject</Button>
      <Button
        variant="primary"
        disabled={hasStale || !anySelected || agent.applying}
        onclick={() => agent.applyProposal(selection(), stagedValues(true))}>Apply</Button
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

  .undone {
    color: var(--text-muted);
    font-weight: var(--font-weight-semibold);
  }
</style>
