<script lang="ts">
  import Button from "$lib/components/shared/ui/Button.svelte";
  import { conflictLines } from "$lib/utils/changeLog";
  import type { UndoConflict } from "$lib/services/changeLog";

  let {
    conflicts,
    disabled,
    oncancel,
    onforce,
  }: {
    conflicts: UndoConflict[];
    disabled: boolean;
    oncancel: () => void;
    onforce: () => void;
  } = $props();
</script>

<div class="conflict">
  <div>These values changed since this entry:</div>
  <ul>
    {#each conflictLines(conflicts) as line}<li>{line}</li>{/each}
  </ul>
  <div class="actions">
    <Button variant="ghost" onclick={oncancel}>Cancel</Button>
    <Button variant="danger" {disabled} onclick={onforce}>Undo anyway</Button>
  </div>
</div>

<style>
  .conflict {
    display: flex;
    flex-direction: column;
    gap: var(--space-2);
    font-size: var(--font-size-sm);
    color: var(--warning);
  }

  .conflict ul {
    margin: 0;
    padding-left: var(--space-5);
    word-break: break-word;
  }

  .actions {
    display: flex;
    gap: var(--space-3);
  }
</style>
