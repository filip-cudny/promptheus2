use std::future::Future;
use std::sync::Arc;

use serde::Serialize;
use tauri::{AppHandle, Emitter, Manager};
use tokio::sync::{watch, Mutex};

use super::session::{load_stt_runtime, MIN_RECORDING_SAMPLES_SECS};
use super::widget::{elapsed_ms, level_from_rms, smooth_level, TICK};
use super::{
    encode_wav, lock_active_session, tail_rms, transcribe_with_retry, ComposerOwner, SpeechError,
    SpeechService,
};
use crate::models::speech::{
    ComposerDictationError, ComposerDictationLevel, ComposerDictationTab,
    ComposerDictationTranscript, DictationBusy,
};
use crate::services::config::ConfigService;

type SharedSpeech = Arc<Mutex<SpeechService>>;

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(untagged)]
pub enum ComposerEvent {
    Level(ComposerDictationLevel),
    Transcribing(ComposerDictationTab),
    Transcript(ComposerDictationTranscript),
    Error(ComposerDictationError),
    Cancelled(ComposerDictationTab),
}

impl ComposerEvent {
    fn name(&self) -> &'static str {
        match self {
            Self::Level(_) => "composer-dictation-level",
            Self::Transcribing(_) => "composer-dictation-transcribing",
            Self::Transcript(_) => "composer-dictation-transcript",
            Self::Error(_) => "composer-dictation-error",
            Self::Cancelled(_) => "composer-dictation-cancelled",
        }
    }
}

/// The only side effects of a composer session: events to its owner window, the
/// app-wide busy state and the transcription itself.
pub trait ComposerEffects: Clone + Send + Sync + 'static {
    fn emit(&self, label: &str, event: ComposerEvent);
    fn busy_changed(&self, busy: Option<DictationBusy>);
    fn transcribe(
        &self,
        wav: Vec<u8>,
        abort: watch::Receiver<bool>,
    ) -> impl Future<Output = Result<String, SpeechError>> + Send;
}

pub struct Job {
    owner: ComposerOwner,
    session: u64,
    wav: Vec<u8>,
}

enum Outcome {
    Transcript(String),
    Failed { message: String, wav: Option<Vec<u8>> },
}

fn tab(owner: &ComposerOwner) -> ComposerDictationTab {
    ComposerDictationTab { tab_id: owner.tab_id.clone() }
}

fn spawn_level_task<E: ComposerEffects>(
    effects: E,
    speech: SharedSpeech,
    owner: ComposerOwner,
    session: u64,
) {
    tokio::spawn(async move {
        let Some(buffer) = lock_active_session(&speech, session).await.map(|s| s.audio_buffer()) else {
            return;
        };
        let mut cursor = 0usize;
        let mut smoothed = 0.0;

        loop {
            tokio::time::sleep(TICK).await;
            let Some(elapsed) = lock_active_session(&speech, session)
                .await
                .map(|s| s.elapsed().map_or(0, elapsed_ms))
            else {
                return;
            };
            smoothed = smooth_level(smoothed, level_from_rms(tail_rms(&buffer, &mut cursor)));
            effects.emit(
                &owner.label,
                ComposerEvent::Level(ComposerDictationLevel {
                    tab_id: owner.tab_id.clone(),
                    level: smoothed,
                    elapsed_ms: elapsed,
                }),
            );
        }
    });
}

pub async fn start<E: ComposerEffects>(
    effects: &E,
    speech: &SharedSpeech,
    owner: ComposerOwner,
) -> Result<(), SpeechError> {
    let (session, busy) = {
        let mut s = speech.lock().await;
        s.start_composer_recording(owner.clone())?;
        (s.session(), s.busy())
    };
    log::info!("composer dictation: started, window={} session={session}", owner.label);
    effects.busy_changed(busy);
    spawn_level_task(effects.clone(), Arc::clone(speech), owner, session);
    Ok(())
}

/// Stops the recording of `label`; `None` when there is nothing to transcribe, after the
/// empty transcript was sent.
pub async fn finish<E: ComposerEffects>(
    effects: &E,
    speech: &SharedSpeech,
    label: &str,
) -> Result<Option<Job>, SpeechError> {
    let (owner, session, audio, busy) = {
        let mut s = speech.lock().await;
        let owner = s
            .composer_session(label)
            .filter(|_| s.is_recording())
            .ok_or(SpeechError::NotRecording)?;
        let (samples, sample_rate) = s.stop_recording_raw()?;
        let min_samples = (sample_rate as f64 * MIN_RECORDING_SAMPLES_SECS) as usize;
        let audio = (samples.len() >= min_samples).then_some((samples, sample_rate));
        if audio.is_some() {
            s.set_transcribing(true);
        } else {
            s.release_composer();
        }
        (owner, s.session(), audio, s.busy())
    };

    let Some((samples, sample_rate)) = audio else {
        log::debug!("composer dictation: recording too short, nothing to transcribe");
        effects.emit(
            &owner.label,
            ComposerEvent::Transcript(ComposerDictationTranscript {
                tab_id: owner.tab_id.clone(),
                text: String::new(),
            }),
        );
        effects.busy_changed(busy);
        return Ok(None);
    };

    log::info!(
        "composer dictation: finished, window={} samples={}",
        owner.label,
        samples.len()
    );
    let encoded = tokio::task::spawn_blocking(move || encode_wav(&samples, sample_rate)).await;
    match encoded {
        Ok(Ok(wav)) => Ok(Some(Job { owner, session, wav })),
        Ok(Err(e)) => {
            log::warn!("composer dictation: encoding the audio failed: {e}");
            conclude(effects, speech, &owner, session, Outcome::Failed { message: e.to_string(), wav: None }).await;
            Ok(None)
        }
        Err(e) => {
            log::warn!("composer dictation: encoding task failed: {e}");
            conclude(effects, speech, &owner, session, Outcome::Failed { message: e.to_string(), wav: None }).await;
            Ok(None)
        }
    }
}

pub async fn retry<E: ComposerEffects>(
    effects: &E,
    speech: &SharedSpeech,
    label: &str,
) -> Result<Job, SpeechError> {
    let (owner, session, wav, busy) = {
        let mut s = speech.lock().await;
        let (owner, session, wav) = s.begin_held_transcription(label)?;
        (owner, session, wav, s.busy())
    };
    log::info!("composer dictation: retry, window={}", owner.label);
    effects.busy_changed(busy);
    Ok(Job { owner, session, wav })
}

pub async fn run<E: ComposerEffects>(effects: &E, speech: &SharedSpeech, job: Job) {
    let Job { owner, session, wav } = job;
    let (abort_tx, abort_rx) = watch::channel(false);
    {
        let mut s = speech.lock().await;
        if !s.owns_composer_transcription(&owner, session) {
            return;
        }
        s.arm_retry_abort(abort_tx);
    }
    effects.emit(&owner.label, ComposerEvent::Transcribing(tab(&owner)));

    let outcome = match effects.transcribe(wav.clone(), abort_rx).await {
        Ok(text) => Outcome::Transcript(text),
        Err(SpeechError::NoSpeechDetected) => Outcome::Transcript(String::new()),
        Err(e) => {
            log::warn!("composer dictation: transcription failed: {e}");
            Outcome::Failed { message: e.to_string(), wav: Some(wav) }
        }
    };
    conclude(effects, speech, &owner, session, outcome).await;
}

async fn conclude<E: ComposerEffects>(
    effects: &E,
    speech: &SharedSpeech,
    owner: &ComposerOwner,
    session: u64,
    outcome: Outcome,
) {
    let (event, busy) = {
        let mut s = speech.lock().await;
        if !s.owns_composer_transcription(owner, session) {
            log::debug!("composer dictation: result dropped, the session ended");
            return;
        }
        let event = match outcome {
            Outcome::Transcript(text) => ComposerEvent::Transcript(ComposerDictationTranscript {
                tab_id: owner.tab_id.clone(),
                text,
            }),
            Outcome::Failed { message, wav } => {
                if let Some(wav) = wav {
                    s.hold_recording(owner, wav);
                }
                ComposerEvent::Error(ComposerDictationError { tab_id: owner.tab_id.clone(), message })
            }
        };
        s.release_composer();
        (event, s.busy())
    };
    effects.emit(&owner.label, event);
    effects.busy_changed(busy);
}

pub async fn cancel<E: ComposerEffects>(effects: &E, speech: &SharedSpeech, label: &str) {
    let (cancelled, busy) = {
        let mut s = speech.lock().await;
        (s.cancel_composer(label), s.busy())
    };
    if let Some(owner) = cancelled {
        log::info!("composer dictation: cancelled, window={label}");
        effects.emit(&owner.label, ComposerEvent::Cancelled(tab(&owner)));
        effects.busy_changed(busy);
    }
}

pub async fn dismiss(speech: &SharedSpeech, label: &str) {
    speech.lock().await.drop_held_recording(label);
}

#[derive(Clone)]
pub struct AppEffects(pub AppHandle);

impl ComposerEffects for AppEffects {
    fn emit(&self, label: &str, event: ComposerEvent) {
        if let Err(e) = self.0.emit_to(label, event.name(), &event) {
            log::debug!("composer dictation: emitting {} failed: {e}", event.name());
        }
    }

    fn busy_changed(&self, busy: Option<DictationBusy>) {
        if let Err(e) = self.0.emit("dictation-busy-changed", busy) {
            log::debug!("composer dictation: emitting the busy state failed: {e}");
        }
    }

    async fn transcribe(
        &self,
        wav: Vec<u8>,
        abort: watch::Receiver<bool>,
    ) -> Result<String, SpeechError> {
        let config = self.0.state::<Arc<Mutex<ConfigService>>>();
        let runtime = load_stt_runtime(&config).await?;
        transcribe_with_retry(
            wav,
            &runtime.model,
            &runtime.options,
            runtime.auto_retry_attempts,
            Some(abort),
            |_, _| async {},
        )
        .await
    }
}

pub async fn emit_busy_changed(app: &AppHandle) {
    let Some(speech) = app.try_state::<SharedSpeech>() else {
        return;
    };
    let busy = speech.lock().await.busy();
    AppEffects(app.clone()).busy_changed(busy);
}

pub fn cancel_owned_by(app: &AppHandle, label: &'static str) {
    let Some(speech) = app.try_state::<SharedSpeech>() else {
        return;
    };
    let speech = Arc::clone(&*speech);
    let effects = AppEffects(app.clone());
    tauri::async_runtime::spawn(async move { cancel(&effects, &speech, label).await });
}

#[cfg(test)]
mod tests {
    use std::collections::VecDeque;
    use std::sync::atomic::AtomicBool;
    use std::sync::Mutex as StdMutex;
    use std::time::Duration;

    use super::super::{RecorderStream, SessionOwner};
    use super::*;
    use crate::models::speech::DictationBy;
    use crate::services::dialog::{CONVERSATION_DIALOG_LABEL, SETTINGS_DIALOG_LABEL};

    enum Reply {
        Text(&'static str),
        Fail(&'static str),
        NoSpeech,
        UntilAborted,
    }

    #[derive(Clone, Default)]
    struct FakeEffects {
        events: Arc<StdMutex<Vec<(String, ComposerEvent)>>>,
        busy: Arc<StdMutex<Vec<Option<DictationBusy>>>>,
        replies: Arc<StdMutex<VecDeque<Reply>>>,
        transcribed: Arc<StdMutex<Vec<Vec<u8>>>>,
    }

    impl FakeEffects {
        fn replying(replies: Vec<Reply>) -> Self {
            Self { replies: Arc::new(StdMutex::new(replies.into())), ..Self::default() }
        }

        fn events(&self) -> Vec<ComposerEvent> {
            self.events.lock().unwrap().iter().map(|(_, event)| event.clone()).collect()
        }

        fn without_levels(&self) -> Vec<ComposerEvent> {
            self.events().into_iter().filter(|e| !matches!(e, ComposerEvent::Level(_))).collect()
        }

        async fn wait_for(&self, found: impl Fn(&[ComposerEvent]) -> bool) {
            for _ in 0..1000 {
                if found(&self.events()) {
                    return;
                }
                tokio::time::sleep(Duration::from_millis(5)).await;
            }
            panic!("event not emitted: {:?}", self.events());
        }
    }

    impl ComposerEffects for FakeEffects {
        fn emit(&self, label: &str, event: ComposerEvent) {
            self.events.lock().unwrap().push((label.to_string(), event));
        }

        fn busy_changed(&self, busy: Option<DictationBusy>) {
            self.busy.lock().unwrap().push(busy);
        }

        async fn transcribe(
            &self,
            wav: Vec<u8>,
            mut abort: watch::Receiver<bool>,
        ) -> Result<String, SpeechError> {
            self.transcribed.lock().unwrap().push(wav);
            let reply = self.replies.lock().unwrap().pop_front().expect("no reply queued");
            match reply {
                Reply::Text(text) => Ok(text.to_string()),
                Reply::Fail(message) => Err(SpeechError::permanent(message)),
                Reply::NoSpeech => Err(SpeechError::NoSpeechDetected),
                Reply::UntilAborted => {
                    let _ = abort.changed().await;
                    Err(SpeechError::Cancelled)
                }
            }
        }
    }

    struct NoStream;

    impl RecorderStream for NoStream {
        fn play(&self) -> Result<(), cpal::PlayStreamError> {
            Ok(())
        }
    }

    fn chat_owner() -> ComposerOwner {
        ComposerOwner::new(CONVERSATION_DIALOG_LABEL, Some("tab-1".into())).unwrap()
    }

    fn speech_recording(owner: ComposerOwner, samples: Vec<i16>) -> SharedSpeech {
        let mut service = SpeechService::new();
        service
            .start_with(None, 16000, Arc::new(StdMutex::new(samples)), Arc::new(AtomicBool::new(false)), || {
                Ok(NoStream)
            })
            .unwrap();
        service.owner = SessionOwner::Composer(owner);
        Arc::new(Mutex::new(service))
    }

    fn speech_with_one_second_of_audio(owner: ComposerOwner) -> SharedSpeech {
        speech_recording(owner, vec![1000; 20_000])
    }

    fn transcript(text: &str) -> ComposerEvent {
        ComposerEvent::Transcript(ComposerDictationTranscript {
            tab_id: Some("tab-1".into()),
            text: text.into(),
        })
    }

    fn cancelled() -> ComposerEvent {
        ComposerEvent::Cancelled(ComposerDictationTab { tab_id: Some("tab-1".into()) })
    }

    fn transcribing() -> ComposerEvent {
        ComposerEvent::Transcribing(ComposerDictationTab { tab_id: Some("tab-1".into()) })
    }

    #[tokio::test]
    async fn a_finished_session_emits_level_and_transcript_only_to_the_owner_window() {
        let fx = FakeEffects::replying(vec![Reply::Text("hello")]);
        let speech = speech_with_one_second_of_audio(chat_owner());
        let session = speech.lock().await.session();

        spawn_level_task(fx.clone(), Arc::clone(&speech), chat_owner(), session);
        fx.wait_for(|events| events.iter().any(|e| matches!(e, ComposerEvent::Level(_)))).await;

        let job = finish(&fx, &speech, CONVERSATION_DIALOG_LABEL).await.unwrap().unwrap();
        run(&fx, &speech, job).await;

        assert!(fx.events.lock().unwrap().iter().all(|(label, _)| label == CONVERSATION_DIALOG_LABEL));
        assert_eq!(fx.without_levels(), vec![transcribing(), transcript("hello")]);
        assert!(fx.transcribed.lock().unwrap()[0].starts_with(b"RIFF"));
        assert_eq!(speech.lock().await.busy(), None);
        assert_eq!(fx.busy.lock().unwrap().last(), Some(&None));
    }

    #[tokio::test]
    async fn the_composer_is_busy_through_the_recording_and_the_transcription() {
        let fx = FakeEffects::replying(vec![Reply::UntilAborted]);
        let speech = speech_with_one_second_of_audio(chat_owner());
        assert_eq!(speech.lock().await.busy().map(|b| b.by), Some(DictationBy::Chat));

        let job = finish(&fx, &speech, CONVERSATION_DIALOG_LABEL).await.unwrap().unwrap();
        assert_eq!(speech.lock().await.busy().map(|b| b.by), Some(DictationBy::Chat));

        let runner = tokio::spawn({
            let (fx, speech) = (fx.clone(), Arc::clone(&speech));
            async move { run(&fx, &speech, job).await }
        });
        fx.wait_for(|events| events.contains(&transcribing())).await;
        cancel(&fx, &speech, CONVERSATION_DIALOG_LABEL).await;
        runner.await.unwrap();
    }

    #[tokio::test]
    async fn finish_by_another_window_is_refused() {
        let fx = FakeEffects::default();
        let speech = speech_with_one_second_of_audio(chat_owner());
        let refused = finish(&fx, &speech, SETTINGS_DIALOG_LABEL).await;
        assert!(matches!(refused, Err(SpeechError::NotRecording)));
        assert!(speech.lock().await.is_recording());
    }

    #[tokio::test]
    async fn cancel_while_recording_discards_the_audio() {
        let fx = FakeEffects::default();
        let speech = speech_with_one_second_of_audio(chat_owner());

        cancel(&fx, &speech, CONVERSATION_DIALOG_LABEL).await;

        assert_eq!(fx.without_levels(), vec![cancelled()]);
        assert!(fx.transcribed.lock().unwrap().is_empty());
        let mut s = speech.lock().await;
        assert_eq!(s.busy(), None);
        assert!(matches!(
            s.begin_held_transcription(CONVERSATION_DIALOG_LABEL),
            Err(SpeechError::AudioUnavailable)
        ));
    }

    #[tokio::test]
    async fn cancel_while_transcribing_aborts_it_and_emits_no_transcript() {
        let fx = FakeEffects::replying(vec![Reply::UntilAborted]);
        let speech = speech_with_one_second_of_audio(chat_owner());
        let job = finish(&fx, &speech, CONVERSATION_DIALOG_LABEL).await.unwrap().unwrap();
        let runner = tokio::spawn({
            let (fx, speech) = (fx.clone(), Arc::clone(&speech));
            async move { run(&fx, &speech, job).await }
        });
        fx.wait_for(|events| events.contains(&transcribing())).await;

        cancel(&fx, &speech, CONVERSATION_DIALOG_LABEL).await;
        runner.await.unwrap();

        assert_eq!(fx.without_levels(), vec![transcribing(), cancelled()]);
        let mut s = speech.lock().await;
        assert_eq!(s.busy(), None);
        assert!(matches!(
            s.begin_held_transcription(CONVERSATION_DIALOG_LABEL),
            Err(SpeechError::AudioUnavailable)
        ));
    }

    #[tokio::test]
    async fn a_cancelled_transcription_does_not_report_into_a_newer_session() {
        let fx = FakeEffects::replying(vec![Reply::UntilAborted]);
        let speech = speech_with_one_second_of_audio(chat_owner());
        let job = finish(&fx, &speech, CONVERSATION_DIALOG_LABEL).await.unwrap().unwrap();
        cancel(&fx, &speech, CONVERSATION_DIALOG_LABEL).await;

        let mut service = speech.lock().await;
        service
            .start_with(None, 16000, Arc::new(StdMutex::new(vec![])), Arc::new(AtomicBool::new(false)), || Ok(NoStream))
            .unwrap();
        service.owner = SessionOwner::Composer(chat_owner());
        service.set_transcribing(true);
        drop(service);

        run(&fx, &speech, job).await;

        assert_eq!(fx.without_levels(), vec![cancelled()]);
        assert!(fx.transcribed.lock().unwrap().is_empty());
    }

    #[tokio::test]
    async fn a_failed_transcription_holds_the_audio_and_retry_transcribes_the_same_bytes() {
        let fx = FakeEffects::replying(vec![Reply::Fail("boom"), Reply::Text("again")]);
        let speech = speech_with_one_second_of_audio(chat_owner());

        let job = finish(&fx, &speech, CONVERSATION_DIALOG_LABEL).await.unwrap().unwrap();
        run(&fx, &speech, job).await;

        let failed = ComposerEvent::Error(ComposerDictationError {
            tab_id: Some("tab-1".into()),
            message: "boom".into(),
        });
        assert_eq!(fx.without_levels(), vec![transcribing(), failed]);
        assert_eq!(speech.lock().await.busy(), None);

        let job = retry(&fx, &speech, CONVERSATION_DIALOG_LABEL).await.unwrap();
        assert!(matches!(
            retry(&fx, &speech, CONVERSATION_DIALOG_LABEL).await,
            Err(SpeechError::Busy(_))
        ));
        run(&fx, &speech, job).await;

        let transcribed = fx.transcribed.lock().unwrap();
        assert_eq!(transcribed.len(), 2);
        assert_eq!(transcribed[0], transcribed[1]);
        drop(transcribed);
        assert_eq!(fx.events().last(), Some(&transcript("again")));
        assert!(matches!(
            retry(&fx, &speech, CONVERSATION_DIALOG_LABEL).await,
            Err(SpeechError::AudioUnavailable)
        ));
    }

    #[tokio::test]
    async fn dismiss_and_a_new_start_drop_the_held_audio() {
        let fx = FakeEffects::replying(vec![Reply::Fail("boom")]);
        let speech = speech_with_one_second_of_audio(chat_owner());
        let job = finish(&fx, &speech, CONVERSATION_DIALOG_LABEL).await.unwrap().unwrap();
        run(&fx, &speech, job).await;

        dismiss(&speech, CONVERSATION_DIALOG_LABEL).await;

        assert!(matches!(
            retry(&fx, &speech, CONVERSATION_DIALOG_LABEL).await,
            Err(SpeechError::AudioUnavailable)
        ));
    }

    #[tokio::test]
    async fn a_too_short_or_empty_recording_emits_an_empty_transcript() {
        for samples in [vec![], vec![1000; 100]] {
            let fx = FakeEffects::default();
            let speech = speech_recording(chat_owner(), samples);

            let job = finish(&fx, &speech, CONVERSATION_DIALOG_LABEL).await.unwrap();

            assert!(job.is_none());
            assert_eq!(fx.without_levels(), vec![transcript("")]);
            assert!(fx.transcribed.lock().unwrap().is_empty());
            assert_eq!(speech.lock().await.busy(), None);
        }
    }

    #[tokio::test]
    async fn no_speech_from_the_transcriber_is_an_empty_transcript_and_holds_nothing() {
        let fx = FakeEffects::replying(vec![Reply::NoSpeech]);
        let speech = speech_with_one_second_of_audio(chat_owner());

        let job = finish(&fx, &speech, CONVERSATION_DIALOG_LABEL).await.unwrap().unwrap();
        run(&fx, &speech, job).await;

        assert_eq!(fx.without_levels(), vec![transcribing(), transcript("")]);
        assert!(matches!(
            retry(&fx, &speech, CONVERSATION_DIALOG_LABEL).await,
            Err(SpeechError::AudioUnavailable)
        ));
    }
}
