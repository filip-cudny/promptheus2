<script lang="ts" generics="T extends string">
  import { ChevronDown } from "lucide-svelte";
  import { ICON_SIZE } from "$lib/constants/ui";
  import FloatingPanel from "./FloatingPanel.svelte";

  let {
    value = $bindable(),
    options,
    ariaLabel,
    activeWhenNot,
  }: {
    value: T;
    options: ReadonlyArray<{ value: T; label: string }>;
    ariaLabel: string;
    activeWhenNot?: T;
  } = $props();

  let triggerEl = $state<HTMLButtonElement | undefined>();
  let open = $state(false);

  let selectedLabel = $derived(
    options.find((o) => o.value === value)?.label ?? "",
  );
  let isActive = $derived(
    activeWhenNot !== undefined && value !== activeWhenNot,
  );

  function toggleOpen() {
    open = !open;
  }

  function handleClose() {
    open = false;
  }

  function selectOption(next: T) {
    value = next;
    open = false;
    triggerEl?.focus();
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

<button
  bind:this={triggerEl}
  type="button"
  class="trigger"
  class:active={isActive}
  aria-haspopup="listbox"
  aria-expanded={open}
  aria-label={ariaLabel}
  onclick={toggleOpen}
>
  <span class="label">{selectedLabel}</span>
  <ChevronDown size={ICON_SIZE.sm} />
</button>

<FloatingPanel
  visible={open}
  anchorEl={triggerEl}
  onclose={handleClose}
  fitContent
>
  <ul class="option-list" role="listbox" aria-label={ariaLabel}>
    {#each options as option (option.value)}
      {@const selected = option.value === value}
      <li
        class="option"
        class:selected
        role="option"
        aria-selected={selected}
        tabindex="0"
        onclick={() => selectOption(option.value)}
        onkeydown={(e) => {
          if (e.key === "Enter" || e.key === " ") {
            e.preventDefault();
            selectOption(option.value);
          }
        }}
      >
        {option.label}
      </li>
    {/each}
  </ul>
</FloatingPanel>

<style>
  .trigger {
    display: inline-flex;
    align-items: center;
    gap: 4px;
    padding: 3px 8px;
    border: 1px solid rgba(255, 255, 255, 0.15);
    border-radius: 4px;
    background: #2a2a2a;
    color: rgba(255, 255, 255, 0.75);
    cursor: pointer;
    font: inherit;
    font-size: 11px;
    line-height: 1.4;
  }

  .trigger:hover {
    background: rgba(255, 255, 255, 0.08);
    color: rgba(255, 255, 255, 0.9);
  }

  .trigger.active {
    background: rgba(100, 160, 255, 0.15);
    border-color: rgba(100, 160, 255, 0.4);
    color: rgba(100, 160, 255, 0.95);
  }

  .trigger.active:hover {
    background: rgba(100, 160, 255, 0.22);
    color: rgba(100, 160, 255, 1);
  }

  .trigger:focus-visible {
    outline: 2px solid rgba(100, 160, 255, 0.6);
    outline-offset: 1px;
  }

  .label {
    white-space: nowrap;
  }

  .option-list {
    list-style: none;
    margin: 0;
    padding: 0;
    min-width: 140px;
    color: #e0e0e0;
    font-size: 12px;
  }

  .option {
    padding: 5px 12px;
    cursor: pointer;
    user-select: none;
    color: rgba(255, 255, 255, 0.85);
  }

  .option:hover {
    background: rgba(255, 255, 255, 0.06);
  }

  .option.selected {
    background: rgba(100, 160, 255, 0.12);
    color: rgba(180, 210, 255, 0.95);
  }

  .option:focus-visible {
    outline: 1px solid rgba(100, 160, 255, 0.5);
    outline-offset: -1px;
  }
</style>
