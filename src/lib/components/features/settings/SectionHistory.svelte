<script lang="ts">
  import { HardDrive, Trash2 } from "lucide-svelte";
  import { ICON_SIZE } from "$lib/constants/ui";
  import SettingsSection from "./SettingsSection.svelte";
  import FormRow from "$lib/components/shared/ui/FormRow.svelte";
  import NumberInput from "$lib/components/shared/ui/NumberInput.svelte";
  import SelectDropdown from "$lib/components/shared/ui/SelectDropdown.svelte";
  import Button from "$lib/components/shared/ui/Button.svelte";
  import ConfirmDialog from "$lib/components/shared/ui/ConfirmDialog.svelte";
  import { getSettingsStore } from "$lib/stores/settings.svelte";
  import { updateHistoryRetention } from "$lib/services/settings";
  import {
    compactHistoryDatabase,
    getHistoryStorageStats,
  } from "$lib/services/history";
  import type { HistoryStorageStats } from "$lib/types";
  import { listen } from "@tauri-apps/api/event";
  import { error as logError } from "@tauri-apps/plugin-log";

  const MAX_RETENTION_DAYS = 3650;
  const FOREVER = "0";
  const CUSTOM = "custom";

  const PRESETS: ReadonlyArray<{ value: string; label: string }> = [
    { value: FOREVER, label: "Keep forever" },
    { value: "7", label: "7 days" },
    { value: "30", label: "30 days" },
    { value: "90", label: "90 days" },
    { value: "365", label: "1 year" },
    { value: CUSTOM, label: "Custom…" },
  ];

  const store = getSettingsStore();
  const savedDays = $derived(store.settings?.history_retention_days ?? 0);
  const matchesPreset = $derived(
    PRESETS.some((p) => p.value === String(savedDays)),
  );

  let customMode = $state(false);
  let customDays = $state(30);
  let pendingDays = $state<number | null>(null);
  let stats = $state<HistoryStorageStats | null>(null);
  let compacting = $state(false);

  const reclaimable = $derived(stats?.reclaimable_bytes ?? 0);
  const worthCompacting = $derived(reclaimable > 1024 * 1024);

  function formatBytes(bytes: number): string {
    if (bytes < 1024) return `${bytes} B`;
    const units = ["KB", "MB", "GB"];
    let value = bytes / 1024;
    let unit = 0;
    while (value >= 1024 && unit < units.length - 1) {
      value /= 1024;
      unit += 1;
    }
    return `${value < 10 ? value.toFixed(1) : Math.round(value)} ${units[unit]}`;
  }

  async function refreshStats() {
    try {
      stats = await getHistoryStorageStats();
    } catch (e) {
      logError(`get_history_storage_stats failed: ${e}`);
    }
  }

  async function compact() {
    compacting = true;
    try {
      stats = await compactHistoryDatabase();
    } catch (e) {
      logError(`compact_history_database failed: ${e}`);
    } finally {
      compacting = false;
    }
  }

  $effect(() => {
    refreshStats();
    const unlisten = listen("history-changed", () => refreshStats());
    return () => {
      unlisten.then((fn) => fn());
    };
  });

  const selected = $derived(
    customMode || !matchesPreset ? CUSTOM : String(savedDays),
  );

  $effect(() => {
    if (!matchesPreset && savedDays > 0) {
      customMode = true;
      customDays = savedDays;
    }
  });

  function clamp(days: number): number {
    return Math.min(MAX_RETENTION_DAYS, Math.max(0, Math.round(days) || 0));
  }

  async function apply(days: number) {
    try {
      await updateHistoryRetention(days);
    } catch (e) {
      logError(`update_history_retention failed: ${e}`);
    }
  }

  function request(days: number) {
    const next = clamp(days);
    if (next === savedDays) return;
    if (next === 0) {
      apply(0);
      return;
    }
    pendingDays = next;
  }

  function confirmPending() {
    const days = pendingDays;
    pendingDays = null;
    if (days !== null) apply(days);
  }

  function handlePreset(next: string) {
    if (next === CUSTOM) {
      customMode = true;
      customDays = savedDays > 0 ? savedDays : 30;
      return;
    }
    customMode = false;
    request(Number(next));
  }
</script>

<SettingsSection
  title="History"
  hint="How long conversations stay in the local database. Nothing is deleted unless you set a retention window."
>
  {#snippet body()}
    <div class="group">
      <FormRow
        label="Retention"
        hint={savedDays === 0
          ? "History grows without limit. Conversations with images are the bulk of the database size."
          : `Conversations untouched for more than ${savedDays} ${savedDays === 1 ? "day" : "days"} are removed. Opening or editing a conversation resets its clock.`}
      >
        <SelectDropdown
          value={selected}
          options={PRESETS}
          ariaLabel="History retention"
          onchange={handlePreset}
        />
      </FormRow>

      {#if selected === CUSTOM}
        <FormRow
          label="Custom retention (days)"
          hint="0 keeps everything forever. Maximum is 3650 days."
        >
          <div class="custom-row">
            <NumberInput
              bind:value={customDays}
              min={0}
              max={MAX_RETENTION_DAYS}
              step={1}
            />
            <Button
              variant="primary"
              disabled={clamp(customDays) === savedDays}
              onclick={() => request(customDays)}
            >
              Apply
            </Button>
          </div>
        </FormRow>
      {/if}
    </div>

    <div class="group">
      <h3>Storage</h3>
      <p class="muted">
        SQLite does not return disk space after a delete — freed pages are reused
        internally but the file stays as large as it ever was. Compacting rewrites the
        database without them.
      </p>

      {#if stats}
        <dl class="stats">
          <div>
            <dt>Database size</dt>
            <dd>{formatBytes(stats.database_bytes)}</dd>
          </div>
          <div>
            <dt>Reclaimable</dt>
            <dd>{formatBytes(reclaimable)}</dd>
          </div>
        </dl>
      {/if}

      <div class="compact-row">
        <Button
          variant="ghost"
          disabled={compacting || !worthCompacting}
          onclick={compact}
        >
          <HardDrive size={ICON_SIZE.sm} />
          {compacting ? "Compacting…" : "Compact database"}
        </Button>
        {#if compacting}
          <span class="muted">The app cannot write history until this finishes.</span>
        {:else if stats && !worthCompacting}
          <span class="muted">Nothing to reclaim.</span>
        {/if}
      </div>
    </div>
  {/snippet}
</SettingsSection>

<ConfirmDialog
  open={pendingDays !== null}
  message={`Conversations older than ${pendingDays} ${pendingDays === 1 ? "day" : "days"} will be deleted immediately, along with their images. This cannot be undone.`}
  confirmLabel="Apply and prune"
  confirmIcon={Trash2}
  onConfirm={confirmPending}
  onCancel={() => (pendingDays = null)}
/>

<style>
  .group {
    display: flex;
    flex-direction: column;
    gap: var(--space-6);
    max-width: 480px;
  }

  .custom-row {
    display: flex;
    align-items: center;
    gap: var(--space-3);
  }

  .custom-row :global(.number-input) {
    max-width: 140px;
  }

  h3 {
    margin: 0;
    font-size: var(--font-size-md);
    font-weight: var(--font-weight-semibold);
    color: var(--text-primary);
  }

  .muted {
    margin: 0;
    font-size: var(--font-size-sm);
    color: var(--text-muted);
  }

  .stats {
    display: flex;
    gap: var(--space-8);
    margin: 0;
  }

  .stats div {
    display: flex;
    flex-direction: column;
    gap: var(--space-1);
  }

  .stats dt {
    font-size: var(--font-size-sm);
    color: var(--text-muted);
  }

  .stats dd {
    margin: 0;
    font-size: var(--font-size-md);
    font-variant-numeric: tabular-nums;
    color: var(--text-primary);
  }

  .compact-row {
    display: flex;
    align-items: center;
    gap: var(--space-3);
  }
</style>
