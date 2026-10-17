<script lang="ts">
  import MenuList from "$lib/components/shared/ui/MenuList.svelte";
  import MenuItem from "$lib/components/shared/ui/MenuItem.svelte";
  import { ArrowBigUp, MessageSquareShare, Mic, Play } from "lucide-svelte";
  import { ICON_SIZE } from "$lib/constants/ui";

  type MetaEntry = { key: string; value: string };

  type Props = {
    recording: boolean;
    micDisabled: boolean;
    runDisabled: boolean;
    tooltip?: string | null;
    metaEntries: MetaEntry[];
    onOpenInDialog: () => void;
    onRunInDialog: () => void;
    onAlternativeExecute: () => void;
  };

  let {
    recording,
    micDisabled,
    runDisabled,
    tooltip = null,
    metaEntries,
    onOpenInDialog,
    onRunInDialog,
    onAlternativeExecute,
  }: Props = $props();
</script>

<MenuList role="menu" expand>
  <MenuItem label="Open in dialog" onclick={onOpenInDialog}>
    {#snippet icon()}<MessageSquareShare size={ICON_SIZE.md} />{/snippet}
  </MenuItem>
  <MenuItem label="Run in dialog" disabled={runDisabled} onclick={onRunInDialog}>
    {#snippet icon()}<Play size={ICON_SIZE.md} />{/snippet}
    {#snippet shortcut()}<ArrowBigUp size={14} strokeWidth={2.25} />{/snippet}
  </MenuItem>
  <MenuItem
    label={recording ? "Stop recording" : "Run with transcription"}
    disabled={micDisabled}
    onclick={onAlternativeExecute}
  >
    {#snippet icon()}<Mic size={ICON_SIZE.md} />{/snippet}
  </MenuItem>
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
