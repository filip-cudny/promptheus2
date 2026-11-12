<script lang="ts">
  import { Check, Copy, Pencil, Save } from "lucide-svelte";
  import ActionIconButton from "$lib/components/shared/ui/ActionIconButton.svelte";

  let {
    lineCount,
    tokenCount,
    editMode = $bindable(false),
    saveDisabled = false,
    onsave,
    oncopy,
  }: {
    lineCount: number;
    tokenCount?: number;
    editMode?: boolean;
    saveDisabled?: boolean;
    onsave: () => void;
    oncopy?: () => void | Promise<void>;
  } = $props();

</script>

<div class="editor-toolbar">
  <span class="line-count">{lineCount} {lineCount === 1 ? "line" : "lines"}{#if tokenCount !== undefined} · ~{tokenCount} tokens{/if}</span>
  <div class="spacer"></div>
  {#if oncopy}
    <ActionIconButton icon={Copy} confirmIcon={Check} onclick={() => oncopy?.()} label="Copy" />
  {/if}
  <ActionIconButton
    icon={Pencil}
    active={editMode}
    onclick={() => (editMode = !editMode)}
    label={editMode ? "View" : "Edit"}
  />
  <ActionIconButton icon={Save} label="Save" disabled={saveDisabled} onclick={onsave} />
</div>

<style>
  .editor-toolbar {
    display: flex;
    align-items: center;
    gap: var(--space-2);
    padding: var(--space-3) var(--space-4);
    border-bottom: 1px solid var(--border-default);
    flex-shrink: 0;
  }

  .line-count {
    font-size: var(--font-size-sm);
    color: var(--text-disabled);
    user-select: none;
  }

  .spacer {
    flex: 1;
  }
</style>
