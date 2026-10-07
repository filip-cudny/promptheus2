<script lang="ts">
  import SettingsSection from "./SettingsSection.svelte";
  import FormRow from "$lib/components/shared/ui/FormRow.svelte";
  import NumberInput from "$lib/components/shared/ui/NumberInput.svelte";
  import { getSettingsStore } from "$lib/stores/settings.svelte";
  import { updateSpeechToTextConfig } from "$lib/services/settings";
  import { error as logError } from "@tauri-apps/plugin-log";
  import type { SpeechToTextConfig } from "$lib/types";

  const MAX_ATTEMPTS = 10;
  const MAX_RETENTION_HOURS = 72;

  const store = getSettingsStore();
  const config = $derived(store.settings?.surfaces?.speech_to_text ?? null);

  async function save(patch: Partial<SpeechToTextConfig>) {
    if (!config) return;
    try {
      await updateSpeechToTextConfig({ ...config, ...patch });
    } catch (e) {
      logError(`update_speech_to_text_config failed: ${e}`);
    }
  }

  function clampedNumber(e: Event, min: number, max: number): number {
    const raw = Math.round(Number((e.target as HTMLInputElement).value) || 0);
    return Math.min(max, Math.max(min, raw));
  }

  const attemptsHint = $derived(
    config?.auto_retry_attempts && config.auto_retry_attempts > 1
      ? `A rate limit or provider outage is re-sent up to ${config.auto_retry_attempts - 1} more ${config.auto_retry_attempts === 2 ? "time" : "times"} with growing delays. Invalid keys and rejected requests are never retried.`
      : "Background retrying is off — a single failure ends the transcription.",
  );

  const retentionHint = $derived(
    config?.audio_retention_hours && config.audio_retention_hours > 0
      ? `A failed recording stays on disk for ${config.audio_retention_hours}h so you can retry or export it from history. Roughly 2 MB per minute of audio.`
      : "Audio is discarded immediately. A failed transcription cannot be retried — you would have to dictate again.",
  );
</script>

<SettingsSection
  title="Speech to text"
  hint="What happens when a transcription fails, and how long the recording survives so you can try again."
>
  {#snippet body()}
    {#if !config}
      <p class="muted">Settings unavailable.</p>
    {:else}
      <div class="group">
        <h3>Recording</h3>

        <label class="check">
          <input
            type="checkbox"
            checked={config.show_recording_widget}
            onchange={(e: Event) =>
              save({ show_recording_widget: (e.target as HTMLInputElement).checked })}
          />
          <span>Show recording widget</span>
        </label>
        <p class="muted">
          A small floating widget shows the recording level and lets you pause, stop or cancel.
          Off shows notifications instead.
        </p>
      </div>

      <div class="group">
        <h3>Failure handling</h3>

        <FormRow label="Transcription attempts" hint={attemptsHint}>
          <NumberInput
            min={1}
            max={MAX_ATTEMPTS}
            step={1}
            value={config.auto_retry_attempts}
            onchange={(e: Event) =>
              save({ auto_retry_attempts: clampedNumber(e, 1, MAX_ATTEMPTS) })}
          />
        </FormRow>

        <FormRow label="Keep audio for (hours)" hint={retentionHint}>
          <NumberInput
            min={0}
            max={MAX_RETENTION_HOURS}
            step={1}
            value={config.audio_retention_hours}
            onchange={(e: Event) =>
              save({ audio_retention_hours: clampedNumber(e, 0, MAX_RETENTION_HOURS) })}
          />
        </FormRow>

        <label class="check">
          <input
            type="checkbox"
            checked={config.keep_audio_on_success}
            onchange={(e: Event) =>
              save({ keep_audio_on_success: (e.target as HTMLInputElement).checked })}
          />
          <span>Keep audio after a successful transcription</span>
        </label>
        <p class="muted">
          Off by default — a successful transcription deletes its recording immediately.
          Turn on only if you want to export the audio of notes that transcribed fine.
        </p>
      </div>
    {/if}
  {/snippet}
</SettingsSection>

<style>
  .group {
    display: flex;
    flex-direction: column;
    gap: var(--space-4);
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
