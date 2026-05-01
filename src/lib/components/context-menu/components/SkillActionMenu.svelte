<script lang="ts">
  import MenuList from "$lib/components/ui/MenuList.svelte";
  import { ArrowBigUp, MessageSquareShare, Mic } from "lucide-svelte";
  import { ICON_SIZE } from "$lib/constants/ui";

  type MetaEntry = { key: string; value: string };

  type Props = {
    recording: boolean;
    micDisabled: boolean;
    tooltip?: string | null;
    metaEntries: MetaEntry[];
    onOpenInDialog: () => void;
    onAlternativeExecute: () => void;
  };

  let {
    recording,
    micDisabled,
    tooltip = null,
    metaEntries,
    onOpenInDialog,
    onAlternativeExecute,
  }: Props = $props();
</script>

<MenuList role="menu" expand>
  <button
    type="button"
    role="menuitem"
    class="menu-list-item"
    onclick={onOpenInDialog}
  >
    <MessageSquareShare size={ICON_SIZE.md} />
    <span class="menu-list-label">Open in dialog</span>
  </button>
  <button
    type="button"
    role="menuitem"
    class="menu-list-item"
    disabled={micDisabled}
    onclick={onAlternativeExecute}
  >
    <Mic size={ICON_SIZE.md} />
    <span class="menu-list-label">
      {recording ? "Stop recording" : "Run with transcription"}
    </span>
    <span class="menu-list-shortcut"><ArrowBigUp size={14} strokeWidth={2.25} /></span>
  </button>
  {#if tooltip || metaEntries.length > 0}
    <div class="menu-list-separator"></div>
    {#if tooltip}
      <div class="menu-list-info">{tooltip}</div>
    {/if}
    {#if metaEntries.length > 0}
      <div class="menu-list-meta-group">
        {#each metaEntries as entry (entry.key)}
          <div class="menu-list-meta">
            <span class="menu-list-meta-key">{entry.key}</span>
            <span class="menu-list-meta-value">{entry.value}</span>
          </div>
        {/each}
      </div>
    {/if}
  {/if}
</MenuList>
