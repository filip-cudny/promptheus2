<script lang="ts">
  import SettingsSection from "./SettingsSection.svelte";
  import FormRow from "$lib/components/shared/ui/FormRow.svelte";
  import NumberInput from "$lib/components/shared/ui/NumberInput.svelte";
  import { getSettingsStore } from "$lib/stores/settings.svelte";
  import { updateNotifications } from "$lib/services/settings";
  import { error as logError } from "@tauri-apps/plugin-log";
  import type { NotificationEvents, NotificationSettings } from "$lib/types";

  const EVENT_LABELS: ReadonlyArray<{ key: keyof NotificationEvents; label: string }> = [
    { key: "prompt_execution_success", label: "Prompt finished" },
    { key: "prompt_execution_cancel", label: "Prompt cancelled" },
    { key: "prompt_execution_in_progress", label: "Prompt already running" },
    { key: "speech_recording_start", label: "Recording started" },
    { key: "speech_recording_stop", label: "Recording stopped" },
    { key: "speech_transcription_success", label: "Transcription finished" },
    { key: "context_saved", label: "Context saved" },
    { key: "context_set", label: "Context set" },
    { key: "context_append", label: "Context appended" },
    { key: "context_cleared", label: "Context cleared" },
    { key: "clipboard_copy", label: "Copied to clipboard" },
    { key: "image_added", label: "Image added" },
  ];

  const store = getSettingsStore();
  const notifications = $derived(store.settings?.notifications ?? null);
  const reminder = $derived(notifications?.recording_reminder ?? null);

  async function save(patch: Partial<NotificationSettings>) {
    if (!notifications) return;
    try {
      await updateNotifications({ ...notifications, ...patch });
    } catch (e) {
      logError(`update_notifications failed: ${e}`);
    }
  }

  function saveReminder(patch: Partial<NonNullable<typeof reminder>>) {
    if (!reminder) return;
    save({ recording_reminder: { ...reminder, ...patch } });
  }

  function saveEvent(key: keyof NotificationEvents, value: boolean) {
    if (!notifications) return;
    save({ events: { ...notifications.events, [key]: value } });
  }

  function seconds(e: Event): number {
    return Math.max(1, Math.round(Number((e.target as HTMLInputElement).value) || 0));
  }
</script>

<SettingsSection
  title="Notifications"
  hint="Which events raise a toast, and how the app reminds you about a running recording."
>
  {#snippet body()}
    {#if !notifications || !reminder}
      <p class="muted">Settings unavailable.</p>
    {:else}
      <div class="group">
        <h3>Recording reminder</h3>
        <p class="muted">
          Warns you that speech recording is still running — either after a stretch of
          silence, or once the hard interval elapses. Guards against a recording started
          by accident or left running after you stopped dictating.
        </p>

        <label class="check">
          <input
            type="checkbox"
            checked={reminder.enabled}
            onchange={(e: Event) => saveReminder({ enabled: (e.target as HTMLInputElement).checked })}
          />
          <span>Remind me while a recording is running</span>
        </label>

        <div class="fields" class:disabled={!reminder.enabled}>
          <FormRow
            label="Silence trigger after (seconds)"
            hint="Minimum recording length before a silence-based reminder can fire."
          >
            <NumberInput
              min={5}
              step={5}
              value={reminder.silence_after_secs}
              disabled={!reminder.enabled}
              onchange={(e: Event) => saveReminder({ silence_after_secs: seconds(e) })}
            />
          </FormRow>

          <FormRow
            label="Silence length (seconds)"
            hint="How long the microphone must stay quiet to count as 'stopped dictating'."
          >
            <NumberInput
              min={2}
              step={1}
              value={reminder.silence_window_secs}
              disabled={!reminder.enabled}
              onchange={(e: Event) => saveReminder({ silence_window_secs: seconds(e) })}
            />
          </FormRow>

          <FormRow
            label="Maximum interval (seconds)"
            hint="Reminder fires regardless of silence once this much time passes since the recording started or since the last reminder."
          >
            <NumberInput
              min={10}
              step={10}
              value={reminder.max_interval_secs}
              disabled={!reminder.enabled}
              onchange={(e: Event) => saveReminder({ max_interval_secs: seconds(e) })}
            />
          </FormRow>
        </div>
      </div>

      <div class="group">
        <h3>Events</h3>
        <div class="checks">
          {#each EVENT_LABELS as event (event.key)}
            <label class="check">
              <input
                type="checkbox"
                checked={notifications.events[event.key]}
                onchange={(e) => saveEvent(event.key, (e.target as HTMLInputElement).checked)}
              />
              <span>{event.label}</span>
            </label>
          {/each}
        </div>
      </div>

      <div class="group">
        <h3>Appearance</h3>
        <label class="check">
          <input
            type="checkbox"
            checked={notifications.monochromatic_notification_icons}
            onchange={(e) =>
              save({
                monochromatic_notification_icons: (e.target as HTMLInputElement).checked,
              })}
          />
          <span>Monochromatic icons</span>
        </label>
      </div>
    {/if}
  {/snippet}
</SettingsSection>

<style>
  .group {
    display: flex;
    flex-direction: column;
    gap: var(--space-3);
    max-width: 560px;
  }

  h3 {
    margin: 0;
    font-size: var(--font-size-xs);
    font-weight: var(--font-weight-semibold);
    text-transform: uppercase;
    letter-spacing: var(--tracking-label);
    color: var(--text-disabled);
  }

  .muted {
    margin: 0;
    font-size: var(--font-size-sm);
    color: var(--text-muted);
  }

  .fields {
    display: flex;
    flex-direction: column;
    gap: var(--space-4);
    padding-left: var(--space-6);
  }

  .fields.disabled {
    opacity: var(--opacity-disabled);
  }

  .checks {
    display: grid;
    grid-template-columns: repeat(2, minmax(0, 1fr));
    gap: var(--space-2) var(--space-4);
  }

  .check {
    display: flex;
    align-items: center;
    gap: var(--space-2);
    font-size: var(--font-size-md);
    color: var(--text-primary);
    cursor: pointer;
  }

  .check input {
    cursor: pointer;
  }
</style>
