<script lang="ts">
  import { ChevronDown, X, Search } from "lucide-svelte";
  import { ICON_SIZE } from "$lib/constants/ui";
  import FloatingPanel from "$lib/components/ui/FloatingPanel.svelte";
  import type { HistorySearchStore } from "$lib/stores/historySearch.svelte";

  let { searchStore }: { searchStore: HistorySearchStore } = $props();

  let triggerEl = $state<HTMLButtonElement | undefined>();
  let open = $state(false);
  let localFilter = $state("");

  let selectedCount = $derived(searchStore.skillFilter.size);
  let selectedFirstName = $derived.by(() => {
    if (selectedCount !== 1) return null;
    const [id] = Array.from(searchStore.skillFilter);
    return searchStore.availableSkills.find((s) => s.skill_id === id)?.skill_name ?? id;
  });

  let visibleSkills = $derived.by(() => {
    const q = localFilter.trim().toLowerCase();
    if (!q) return searchStore.availableSkills;
    return searchStore.availableSkills.filter((s) =>
      s.skill_name.toLowerCase().includes(q),
    );
  });

  let showLocalSearch = $derived(searchStore.availableSkills.length > 10);

  function toggleOpen() {
    open = !open;
    if (open) localFilter = "";
  }

  function handleClose() {
    open = false;
  }

  function handleClearAllSkills(e: MouseEvent) {
    e.stopPropagation();
    searchStore.clearSkills();
  }

  function handleKeydown(e: KeyboardEvent) {
    if (e.key === "Escape" && open) {
      e.preventDefault();
      open = false;
      triggerEl?.focus();
    }
  }
</script>

<svelte:window onkeydown={handleKeydown} />

<div class="trigger-group" class:active={selectedCount > 0}>
  <button
    bind:this={triggerEl}
    type="button"
    class="trigger"
    aria-haspopup="listbox"
    aria-expanded={open}
    onclick={toggleOpen}
  >
    {#if selectedCount === 0}
      <span class="label">Skills</span>
      <ChevronDown size={ICON_SIZE.sm} />
    {:else if selectedCount === 1}
      <span class="label">Skill: {selectedFirstName}</span>
    {:else}
      <span class="label">Skills ({selectedCount})</span>
    {/if}
  </button>
  {#if selectedCount > 0}
    <button
      type="button"
      class="clear-icon"
      onclick={handleClearAllSkills}
      title={selectedCount === 1 ? "Clear skill" : "Clear skills"}
      aria-label="Clear skill filter"
    >
      <X size={ICON_SIZE.sm} />
    </button>
  {/if}
</div>

<FloatingPanel
  visible={open}
  anchorEl={triggerEl}
  onclose={handleClose}
  fitContent
>
  <div class="popover">
    {#if showLocalSearch}
      <div class="local-search">
        <span class="local-search-icon"><Search size={ICON_SIZE.sm} /></span>
        <input
          type="search"
          class="local-search-input"
          bind:value={localFilter}
          placeholder="Filter skills..."
          autocomplete="off"
          spellcheck="false"
        />
      </div>
    {/if}

    <ul
      class="skill-list"
      role="listbox"
      aria-multiselectable="true"
      aria-label="Skills"
    >
      {#each visibleSkills as skill (skill.skill_id)}
        {@const selected = searchStore.skillFilter.has(skill.skill_id)}
        <li
          class="skill-item"
          class:selected
          role="option"
          aria-selected={selected}
          tabindex="0"
          onclick={() => searchStore.toggleSkill(skill.skill_id)}
          onkeydown={(e) => {
            if (e.key === "Enter" || e.key === " ") {
              e.preventDefault();
              searchStore.toggleSkill(skill.skill_id);
            }
          }}
        >
          <span class="checkbox" aria-hidden="true">
            {#if selected}<span class="check">✓</span>{/if}
          </span>
          <span class="skill-name">{skill.skill_name}</span>
          <span class="skill-count">({skill.count})</span>
        </li>
      {:else}
        <li class="empty">No skills match</li>
      {/each}
    </ul>

    <div class="footer">
      <button
        type="button"
        class="footer-btn"
        disabled={selectedCount === 0}
        onclick={() => searchStore.clearSkills()}
      >
        Clear
      </button>
      <button type="button" class="footer-btn primary" onclick={handleClose}>
        Done
      </button>
    </div>
  </div>
</FloatingPanel>

<style>
  .trigger-group {
    display: inline-flex;
    align-items: stretch;
    border: 1px solid rgba(255, 255, 255, 0.15);
    border-radius: 4px;
    overflow: hidden;
    color: rgba(255, 255, 255, 0.55);
    line-height: 1.4;
  }

  .trigger-group.active {
    background: rgba(100, 160, 255, 0.15);
    border-color: rgba(100, 160, 255, 0.4);
    color: rgba(100, 160, 255, 0.95);
  }

  .trigger {
    display: inline-flex;
    align-items: center;
    gap: 4px;
    padding: 3px 8px;
    border: none;
    background: transparent;
    color: inherit;
    cursor: pointer;
    font: inherit;
    font-size: 11px;
  }

  .trigger:hover {
    background: rgba(255, 255, 255, 0.08);
    color: rgba(255, 255, 255, 0.85);
  }

  .trigger-group.active .trigger:hover {
    background: rgba(100, 160, 255, 0.22);
    color: rgba(100, 160, 255, 1);
  }

  .label {
    white-space: nowrap;
  }

  .clear-icon {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    padding: 0 4px;
    border: none;
    border-left: 1px solid rgba(100, 160, 255, 0.25);
    background: transparent;
    color: inherit;
    cursor: pointer;
  }

  .clear-icon:hover {
    background: rgba(255, 255, 255, 0.15);
  }

  .popover {
    display: flex;
    flex-direction: column;
    min-width: 220px;
    max-width: 320px;
    color: #e0e0e0;
    font-size: 12px;
  }

  .local-search {
    position: relative;
    display: flex;
    align-items: center;
    margin-bottom: 6px;
  }

  .local-search-icon {
    position: absolute;
    left: 8px;
    display: flex;
    align-items: center;
    color: rgba(255, 255, 255, 0.4);
    pointer-events: none;
  }

  .local-search-input {
    flex: 1;
    width: 100%;
    padding: 4px 8px 4px 24px;
    background: #2a2a2a;
    border: 1px solid #3a3a3a;
    border-radius: 4px;
    color: #e0e0e0;
    font: inherit;
    font-size: 12px;
    outline: none;
  }

  .local-search-input::-webkit-search-decoration,
  .local-search-input::-webkit-search-cancel-button {
    appearance: none;
  }

  .local-search-input:focus {
    border-color: rgba(100, 160, 255, 0.5);
  }

  .skill-list {
    list-style: none;
    margin: 0;
    padding: 0;
    max-height: 280px;
    overflow-y: auto;
  }

  .skill-item {
    display: flex;
    align-items: center;
    gap: 8px;
    padding: 5px 8px;
    border-radius: 4px;
    cursor: pointer;
    user-select: none;
  }

  .skill-item:hover {
    background: rgba(255, 255, 255, 0.06);
  }

  .skill-item.selected {
    background: rgba(100, 160, 255, 0.12);
  }

  .skill-item:focus-visible {
    outline: 1px solid rgba(100, 160, 255, 0.5);
    outline-offset: -1px;
  }

  .checkbox {
    flex-shrink: 0;
    display: inline-flex;
    align-items: center;
    justify-content: center;
    width: 14px;
    height: 14px;
    border: 1px solid rgba(255, 255, 255, 0.3);
    border-radius: 3px;
    background: rgba(0, 0, 0, 0.2);
  }

  .skill-item.selected .checkbox {
    background: rgba(100, 160, 255, 0.5);
    border-color: rgba(100, 160, 255, 0.7);
  }

  .check {
    font-size: 10px;
    line-height: 1;
    color: #fff;
  }

  .skill-name {
    flex: 1;
    color: rgba(255, 255, 255, 0.9);
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .skill-count {
    flex-shrink: 0;
    color: rgba(255, 255, 255, 0.4);
    font-size: 11px;
  }

  .empty {
    padding: 8px;
    color: rgba(255, 255, 255, 0.4);
    text-align: center;
    font-style: italic;
  }

  .footer {
    display: flex;
    justify-content: space-between;
    gap: 6px;
    margin-top: 6px;
    padding-top: 6px;
    border-top: 1px solid rgba(255, 255, 255, 0.08);
  }

  .footer-btn {
    padding: 3px 10px;
    border: 1px solid rgba(255, 255, 255, 0.15);
    border-radius: 4px;
    background: transparent;
    color: rgba(255, 255, 255, 0.7);
    cursor: pointer;
    font: inherit;
    font-size: 11px;
  }

  .footer-btn:hover:not(:disabled) {
    background: rgba(255, 255, 255, 0.08);
    color: rgba(255, 255, 255, 0.95);
  }

  .footer-btn:disabled {
    opacity: 0.4;
    cursor: default;
  }

  .footer-btn.primary {
    background: rgba(100, 160, 255, 0.18);
    border-color: rgba(100, 160, 255, 0.4);
    color: rgba(180, 210, 255, 0.95);
  }

  .footer-btn.primary:hover {
    background: rgba(100, 160, 255, 0.28);
  }
</style>
