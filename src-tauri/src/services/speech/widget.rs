use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::time::Duration;

use serde::Serialize;
use tauri::{AppHandle, Emitter, Manager, PhysicalPosition, PhysicalSize};
use tokio::sync::Mutex;

use super::reminder::rms;
use super::SpeechService;
use crate::services::config::ConfigService;
use crate::services::monitor::{clamp_origin, find_monitor_at};
use crate::services::ui_state::UiStateService;

pub const WINDOW_LABEL: &str = "recording-widget";
pub const WINDOW_POSITION_KEY: &str = "recording_widget.window_position";
pub const SHELL_POSITION_KEY: &str = "recording_widget.shell_position";

const STATE_EVENT: &str = "recording-widget-state";
const TICK: Duration = Duration::from_millis(50);
const DONE_VISIBLE: Duration = Duration::from_secs(1);
const MOVE_DEBOUNCE: Duration = Duration::from_millis(500);
const FULL_SCALE_RMS: f64 = 32768.0;
const FLOOR_DB: f64 = -50.0;
const CEIL_DB: f64 = -20.0;
const LEVEL_ATTACK: f64 = 0.6;
const LEVEL_RELEASE: f64 = 0.2;
const BOTTOM_MARGIN: f64 = 24.0;
const PILL_WIDTH: f64 = 240.0;
const PILL_HEIGHT: f64 = 44.0;

type Rect = (i32, i32, u32, u32);
type WorkArea = (f64, f64, f64, f64);

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Placement {
    pub origin: (f64, f64),
    pub work_area: WorkArea,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Transport {
    #[cfg(target_os = "linux")]
    Shell,
    Window,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SessionKind {
    #[cfg(target_os = "linux")]
    GnomeWayland,
    #[cfg(target_os = "linux")]
    OtherWayland,
    X11OrMacos,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum WidgetState {
    Recording,
    Paused,
    Processing,
    Done,
}

impl WidgetState {
    fn as_str(self) -> &'static str {
        match self {
            Self::Recording => "recording",
            Self::Paused => "paused",
            Self::Processing => "processing",
            Self::Done => "done",
        }
    }

    fn is_live(self) -> bool {
        matches!(self, Self::Recording | Self::Paused)
    }
}

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct WidgetPayload {
    state: &'static str,
    level: f64,
    elapsed_ms: u32,
    monochromatic: bool,
}

#[derive(Debug, Clone, Copy, Default)]
struct WidgetSettings {
    enabled: bool,
    monochromatic: bool,
    at_cursor: bool,
}

struct Inner {
    transport: Option<Transport>,
    session: u64,
    state: WidgetState,
    generation: u64,
    elapsed_ms: u32,
    monochromatic: bool,
    update_failed: bool,
    placement: Option<Placement>,
    #[cfg(target_os = "linux")]
    shell: Option<crate::services::gnome_shell::ShellProxy<'static>>,
}

pub struct RecordingWidget {
    inner: Mutex<Inner>,
    move_seq: AtomicU64,
    programmatic_origin: std::sync::Mutex<Option<(i32, i32)>>,
}

pub fn choose_transport(enabled: bool, session: SessionKind) -> Option<Transport> {
    if !enabled {
        return None;
    }
    match session {
        #[cfg(target_os = "linux")]
        SessionKind::GnomeWayland => Some(Transport::Shell),
        #[cfg(target_os = "linux")]
        SessionKind::OtherWayland => None,
        SessionKind::X11OrMacos => Some(Transport::Window),
    }
}

fn current_session_kind() -> SessionKind {
    #[cfg(target_os = "linux")]
    {
        if crate::services::gnome_shell::is_gnome_wayland() {
            return SessionKind::GnomeWayland;
        }
        if std::env::var("XDG_SESSION_TYPE").is_ok_and(|v| v == "wayland") {
            return SessionKind::OtherWayland;
        }
    }
    SessionKind::X11OrMacos
}

pub fn level_from_rms(rms: f64) -> f64 {
    if rms <= 0.0 {
        return 0.0;
    }
    let dbfs = 20.0 * (rms / FULL_SCALE_RMS).log10();
    ((dbfs - FLOOR_DB) / (CEIL_DB - FLOOR_DB)).clamp(0.0, 1.0)
}

pub fn smooth_level(previous: f64, target: f64) -> f64 {
    let rate = if target > previous { LEVEL_ATTACK } else { LEVEL_RELEASE };
    previous + (target - previous) * rate
}

fn default_origin(work_area: Rect, size: (u32, u32), scale: f64) -> (i32, i32) {
    let (x, y, width, height) = work_area;
    let left = x + (width as i32 - size.0 as i32) / 2;
    let top = y + height as i32 - size.1 as i32 - (BOTTOM_MARGIN * scale) as i32;
    (left, top)
}

fn physical_size(logical: (f64, f64), scale: f64) -> (u32, u32) {
    ((logical.0 * scale).round() as u32, (logical.1 * scale).round() as u32)
}

fn should_store_drag(at_cursor: bool) -> bool {
    !at_cursor
}

fn work_area_of(monitor: &tauri::Monitor) -> WorkArea {
    let work = monitor.work_area();
    (
        work.position.x as f64,
        work.position.y as f64,
        work.size.width as f64,
        work.size.height as f64,
    )
}

fn log_placement(source: &str, origin: (f64, f64), size: (f64, f64), work_area: WorkArea) {
    log::debug!(
        "recording widget: placed from {source} origin={origin:?} size={size:?} work_area={work_area:?}"
    );
}

fn is_inside_any(point: (i32, i32), work_areas: &[Rect]) -> bool {
    work_areas.iter().any(|&(x, y, width, height)| {
        point.0 >= x
            && point.0 < x + width as i32
            && point.1 >= y
            && point.1 < y + height as i32
    })
}

fn elapsed_ms(elapsed: Duration) -> u32 {
    elapsed.as_millis().min(u32::MAX as u128) as u32
}

async fn read_position(app: &AppHandle, key: &str) -> Option<(i32, i32)> {
    let ui_state = app.try_state::<Arc<Mutex<UiStateService>>>()?;
    let value = ui_state.lock().await.get(key)?;
    Some((value.get("x")?.as_i64()? as i32, value.get("y")?.as_i64()? as i32))
}

pub async fn store_position(app: &AppHandle, key: &str, x: i32, y: i32) {
    let Some(ui_state) = app.try_state::<Arc<Mutex<UiStateService>>>() else {
        return;
    };
    let stored = ui_state
        .lock()
        .await
        .set(key, serde_json::json!({ "x": x, "y": y }));
    match stored {
        Ok(()) => log::debug!("recording widget position stored: {key}=({x}, {y})"),
        Err(e) => log::warn!("failed to store the recording widget position {key}: {e}"),
    }
}

pub fn window_moved(app: &AppHandle, position: PhysicalPosition<i32>) {
    if let Some(widget) = app.try_state::<RecordingWidget>() {
        widget.record_move(app, (position.x, position.y));
    }
}

async fn widget_settings(app: &AppHandle) -> WidgetSettings {
    let Some(config) = app.try_state::<Arc<Mutex<ConfigService>>>() else {
        return WidgetSettings::default();
    };
    let config = config.lock().await;
    let speech = &config.settings().surfaces.speech_to_text;
    WidgetSettings {
        enabled: speech.show_recording_widget,
        monochromatic: speech.monochromatic_widget_icon,
        at_cursor: speech.recording_widget_at_cursor,
    }
}

#[cfg(target_os = "linux")]
async fn show_in_shell(
    app: &AppHandle,
    settings: WidgetSettings,
) -> Result<(crate::services::gnome_shell::ShellProxy<'static>, Placement), String> {
    let proxy = crate::services::gnome_shell::proxy()
        .await
        .map_err(|e| e.to_string())?;
    let stored = read_position(app, SHELL_POSITION_KEY).await;
    let (x, y, has_position) = if settings.at_cursor {
        let (px, py, work_x, work_y, work_width, work_height) =
            proxy.get_pointer().await.map_err(|e| e.to_string())?;
        let (x, y) = clamp_origin(
            (px as f64, py as f64),
            (PILL_WIDTH, PILL_HEIGHT),
            (work_x as f64, work_y as f64, work_width as f64, work_height as f64),
        );
        (x.round() as i32, y.round() as i32, true)
    } else {
        let (x, y) = stored.unwrap_or((0, 0));
        (x, y, stored.is_some())
    };
    let (x, y, work_x, work_y, work_width, work_height) = proxy
        .show_recording_widget(x, y, has_position, settings.monochromatic)
        .await
        .map_err(|e| e.to_string())?;
    let placement = Placement {
        origin: (x as f64, y as f64),
        work_area: (work_x as f64, work_y as f64, work_width as f64, work_height as f64),
    };
    let source = if settings.at_cursor {
        "cursor"
    } else if has_position && stored == Some((placement.origin.0 as i32, placement.origin.1 as i32)) {
        "stored position"
    } else {
        "default"
    };
    log_placement(source, placement.origin, (PILL_WIDTH, PILL_HEIGHT), placement.work_area);
    Ok((proxy, placement))
}

async fn work_areas(app: &AppHandle) -> Result<Vec<Rect>, String> {
    let monitors = app.available_monitors().map_err(|e| e.to_string())?;
    Ok(monitors
        .iter()
        .map(|m| {
            let work = m.work_area();
            (work.position.x, work.position.y, work.size.width, work.size.height)
        })
        .collect())
}

impl RecordingWidget {
    pub fn new() -> Self {
        Self {
            inner: Mutex::new(Inner {
                transport: None,
                session: 0,
                state: WidgetState::Recording,
                generation: 0,
                elapsed_ms: 0,
                monochromatic: true,
                update_failed: false,
                placement: None,
                #[cfg(target_os = "linux")]
                shell: None,
            }),
            move_seq: AtomicU64::new(0),
            programmatic_origin: std::sync::Mutex::new(None),
        }
    }

    pub async fn is_shown(&self) -> bool {
        self.inner.lock().await.transport.is_some()
    }

    pub async fn show(&self, app: &AppHandle, session: u64) -> bool {
        let settings = widget_settings(app).await;
        let kind = current_session_kind();
        let transport = choose_transport(settings.enabled, kind);
        #[cfg(target_os = "linux")]
        if settings.enabled && kind == SessionKind::OtherWayland {
            log::warn!(
                "recording widget unavailable on this Wayland session without the GNOME Shell extension, using toasts"
            );
        }
        let Some(transport) = transport else {
            return false;
        };

        let mut inner = self.inner.lock().await;
        inner.generation += 1;
        inner.session = session;
        inner.state = WidgetState::Recording;
        inner.elapsed_ms = 0;
        inner.monochromatic = settings.monochromatic;
        inner.update_failed = false;
        inner.placement = None;
        inner.transport = None;
        #[cfg(target_os = "linux")]
        {
            inner.shell = None;
        }

        let shown = match transport {
            #[cfg(target_os = "linux")]
            Transport::Shell => show_in_shell(app, settings).await.map(|(proxy, placement)| {
                inner.shell = Some(proxy);
                inner.placement = Some(placement);
            }),
            Transport::Window => self
                .show_window(app, settings)
                .await
                .map(|placement| inner.placement = Some(placement)),
        };
        if let Err(e) = shown {
            log::warn!("showing the recording widget failed, using toasts: {e}");
            return false;
        }

        log::debug!(
            "recording widget: show session={session} transport={transport:?} monochromatic={}",
            settings.monochromatic
        );
        inner.transport = Some(transport);
        if transport == Transport::Window {
            push(app, &mut inner, 0.0).await;
        }
        spawn_level_task(app.clone(), session, inner.generation);
        true
    }

    pub async fn pause(&self, app: &AppHandle) {
        self.transition(app, "pause", WidgetState::Paused).await;
    }

    pub async fn resume(&self, app: &AppHandle) {
        self.transition(app, "resume", WidgetState::Recording).await;
    }

    pub async fn processing(&self, app: &AppHandle) {
        self.transition(app, "processing", WidgetState::Processing).await;
    }

    pub async fn done(&self, app: &AppHandle) {
        let generation = {
            let mut inner = self.inner.lock().await;
            if inner.transport.is_none() {
                return;
            }
            log::debug!("recording widget: done session={}", inner.session);
            inner.state = WidgetState::Done;
            push(app, &mut inner, 0.0).await;
            inner.generation
        };
        let app = app.clone();
        tauri::async_runtime::spawn(async move {
            tokio::time::sleep(DONE_VISIBLE).await;
            let widget = app.state::<RecordingWidget>();
            let mut inner = widget.inner.lock().await;
            if inner.generation == generation {
                widget.close_locked(&app, &mut inner).await;
            }
        });
    }

    pub async fn close(&self, app: &AppHandle) {
        let mut inner = self.inner.lock().await;
        self.close_locked(app, &mut inner).await;
    }

    async fn transition(&self, app: &AppHandle, name: &str, state: WidgetState) {
        let mut inner = self.inner.lock().await;
        if inner.transport.is_none() {
            return;
        }
        let allowed = match state {
            WidgetState::Paused | WidgetState::Recording => inner.state.is_live(),
            WidgetState::Processing => inner.state != WidgetState::Done,
            WidgetState::Done => true,
        };
        if !allowed {
            return;
        }
        log::debug!("recording widget: {name} session={}", inner.session);
        inner.state = state;
        push(app, &mut inner, 0.0).await;
    }

    async fn close_locked(&self, app: &AppHandle, inner: &mut Inner) {
        let Some(transport) = inner.transport.take() else {
            return;
        };
        log::debug!("recording widget: close session={}", inner.session);
        inner.generation += 1;
        let result = match transport {
            #[cfg(target_os = "linux")]
            Transport::Shell => match inner.shell.take() {
                Some(proxy) => proxy.hide_recording_widget().await.map_err(|e| e.to_string()),
                None => Ok(()),
            },
            Transport::Window => match app.get_webview_window(WINDOW_LABEL) {
                Some(window) => window.hide().map_err(|e| e.to_string()),
                None => Ok(()),
            },
        };
        if let Err(e) = result {
            log::warn!("hiding the recording widget failed: {e}");
        }
    }

    async fn show_window(&self, app: &AppHandle, settings: WidgetSettings) -> Result<Placement, String> {
        let window = app
            .get_webview_window(WINDOW_LABEL)
            .ok_or("recording-widget window not found")?;
        let (origin, work_area, size, source) = if settings.at_cursor {
            let (cx, cy) = app
                .cursor_position()
                .map(|p| (p.x, p.y))
                .unwrap_or((0.0, 0.0));
            let monitor = find_monitor_at(app, cx as i32, cy as i32)?;
            let work_area = work_area_of(&monitor);
            let size = physical_size((PILL_WIDTH, PILL_HEIGHT), monitor.scale_factor());
            window
                .set_size(PhysicalSize::new(size.0, size.1))
                .map_err(|e| e.to_string())?;
            let origin = clamp_origin((cx, cy), (size.0 as f64, size.1 as f64), work_area);
            (origin, work_area, size, "cursor")
        } else {
            let areas = work_areas(app).await?;
            let stored = read_position(app, WINDOW_POSITION_KEY)
                .await
                .filter(|&point| is_inside_any(point, &areas));
            let outer = window.outer_size().map_err(|e| e.to_string())?;
            let size = (outer.width, outer.height);
            match stored {
                Some((x, y)) => {
                    let monitor = find_monitor_at(app, x, y)?;
                    ((x as f64, y as f64), work_area_of(&monitor), size, "stored position")
                }
                None => {
                    let (cx, cy) = app
                        .cursor_position()
                        .map(|p| (p.x as i32, p.y as i32))
                        .unwrap_or((0, 0));
                    let monitor = find_monitor_at(app, cx, cy)?;
                    let work = monitor.work_area();
                    let (x, y) = default_origin(
                        (work.position.x, work.position.y, work.size.width, work.size.height),
                        size,
                        monitor.scale_factor(),
                    );
                    ((x as f64, y as f64), work_area_of(&monitor), size, "default")
                }
            }
        };
        let origin = (origin.0.round(), origin.1.round());
        log_placement(source, origin, (size.0 as f64, size.1 as f64), work_area);
        *self
            .programmatic_origin
            .lock()
            .unwrap_or_else(|e| e.into_inner()) = Some((origin.0 as i32, origin.1 as i32));
        window
            .set_position(PhysicalPosition::new(origin.0 as i32, origin.1 as i32))
            .map_err(|e| e.to_string())?;
        window.show().map_err(|e| e.to_string())?;
        Ok(Placement { origin, work_area })
    }

    fn record_move(&self, app: &AppHandle, position: (i32, i32)) {
        let programmatic = *self
            .programmatic_origin
            .lock()
            .unwrap_or_else(|e| e.into_inner());
        if programmatic == Some(position) {
            return;
        }
        let app = app.clone();
        tauri::async_runtime::spawn(async move {
            let widget = app.state::<RecordingWidget>();
            if let Ok(monitor) = find_monitor_at(&app, position.0, position.1) {
                widget.inner.lock().await.placement = Some(Placement {
                    origin: (position.0 as f64, position.1 as f64),
                    work_area: work_area_of(&monitor),
                });
            }
            widget.store_drag_debounced(&app, WINDOW_POSITION_KEY, position).await;
        });
    }

    pub fn shell_moved(&self, app: &AppHandle, placement: Placement) {
        let app = app.clone();
        tauri::async_runtime::spawn(async move {
            let widget = app.state::<RecordingWidget>();
            widget.inner.lock().await.placement = Some(placement);
            let position = (placement.origin.0 as i32, placement.origin.1 as i32);
            widget.store_drag_debounced(&app, SHELL_POSITION_KEY, position).await;
        });
    }

    async fn store_drag_debounced(&self, app: &AppHandle, key: &str, position: (i32, i32)) {
        let seq = self.move_seq.fetch_add(1, Ordering::SeqCst) + 1;
        tokio::time::sleep(MOVE_DEBOUNCE).await;
        if self.move_seq.load(Ordering::SeqCst) == seq
            && should_store_drag(widget_settings(app).await.at_cursor)
        {
            store_position(app, key, position.0, position.1).await;
        }
    }
}

async fn push(app: &AppHandle, inner: &mut Inner, level: f64) {
    let Some(transport) = inner.transport else {
        return;
    };
    let state = inner.state.as_str();
    let result = match transport {
        #[cfg(target_os = "linux")]
        Transport::Shell => match &inner.shell {
            Some(proxy) => proxy
                .update_recording_widget(state, level, inner.elapsed_ms)
                .await
                .map_err(|e| e.to_string()),
            None => Ok(()),
        },
        Transport::Window => app
            .emit_to(
                WINDOW_LABEL,
                STATE_EVENT,
                WidgetPayload {
                    state,
                    level,
                    elapsed_ms: inner.elapsed_ms,
                    monochromatic: inner.monochromatic,
                },
            )
            .map_err(|e| e.to_string()),
    };
    if let Err(e) = result {
        if !inner.update_failed {
            inner.update_failed = true;
            log::warn!("updating the recording widget failed: {e}");
        }
    }
}

fn spawn_level_task(app: AppHandle, session: u64, generation: u64) {
    tauri::async_runtime::spawn(async move {
        let Some(speech) = app
            .try_state::<Arc<Mutex<SpeechService>>>()
            .map(|s| s.inner().clone())
        else {
            return;
        };
        let buffer = {
            let s = speech.lock().await;
            if !s.is_recording() || s.session() != session {
                return;
            }
            s.audio_buffer()
        };
        let widget = app.state::<RecordingWidget>();
        let mut cursor = 0usize;
        let mut smoothed = 0.0;

        loop {
            tokio::time::sleep(TICK).await;

            let elapsed = {
                let s = speech.lock().await;
                if !s.is_recording() || s.session() != session {
                    return;
                }
                s.elapsed().map_or(0, elapsed_ms)
            };
            let level = {
                let buf = buffer.lock().unwrap_or_else(|e| e.into_inner());
                if buf.len() < cursor {
                    cursor = buf.len();
                }
                let tail = &buf[cursor..];
                cursor = buf.len();
                level_from_rms(rms(tail))
            };

            let mut inner = widget.inner.lock().await;
            if inner.generation != generation || !inner.state.is_live() {
                return;
            }
            inner.elapsed_ms = elapsed;
            smoothed = if inner.state == WidgetState::Paused {
                0.0
            } else {
                smooth_level(smoothed, level)
            };
            push(&app, &mut inner, smoothed).await;
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn disabled_widget_has_no_transport_in_any_session() {
        assert_eq!(choose_transport(false, SessionKind::X11OrMacos), None);
        #[cfg(target_os = "linux")]
        {
            assert_eq!(choose_transport(false, SessionKind::GnomeWayland), None);
            assert_eq!(choose_transport(false, SessionKind::OtherWayland), None);
        }
    }

    #[test]
    fn x11_or_macos_uses_the_window() {
        assert_eq!(
            choose_transport(true, SessionKind::X11OrMacos),
            Some(Transport::Window)
        );
    }

    #[cfg(target_os = "linux")]
    #[test]
    fn other_wayland_has_no_transport() {
        assert_eq!(choose_transport(true, SessionKind::OtherWayland), None);
    }

    #[cfg(target_os = "linux")]
    #[test]
    fn gnome_wayland_uses_the_shell() {
        assert_eq!(
            choose_transport(true, SessionKind::GnomeWayland),
            Some(Transport::Shell)
        );
    }

    #[test]
    fn level_maps_rms_to_zero_through_one() {
        let at_dbfs = |db: f64| FULL_SCALE_RMS * 10f64.powf(db / 20.0);
        assert_eq!(level_from_rms(0.0), 0.0);
        assert_eq!(level_from_rms(-1.0), 0.0);
        assert!(level_from_rms(at_dbfs(-50.0)).abs() < 1e-9);
        assert_eq!(level_from_rms(50.0), 0.0);
        assert!((level_from_rms(at_dbfs(-30.0)) - 2.0 / 3.0).abs() < 1e-9);
        assert!((level_from_rms(1036.0) - 0.667).abs() < 1e-3);
        assert_eq!(level_from_rms(at_dbfs(-20.0) + 1e-6), 1.0);
        assert_eq!(level_from_rms(3277.0), 1.0);
        assert_eq!(level_from_rms(32768.0), 1.0);
    }

    #[test]
    fn smoothed_level_rises_fast() {
        assert!(smooth_level(0.0, 1.0) >= 0.6);
        assert!(smooth_level(0.25, 0.75) >= 0.55);
    }

    #[test]
    fn smoothed_level_falls_slowly() {
        assert!(smooth_level(1.0, 0.0) > 0.5);
        let after_eleven_ticks = (0..11).fold(1.0, |level, _| smooth_level(level, 0.0));
        assert!(after_eleven_ticks < 0.1);
        assert!(after_eleven_ticks > 0.0);
    }

    #[test]
    fn smoothed_level_holds_a_steady_target() {
        assert_eq!(smooth_level(0.4, 0.4), 0.4);
    }

    #[test]
    fn point_inside_a_work_area_is_accepted() {
        let areas = [(0, 0, 1920, 1040), (1920, 0, 2560, 1400)];
        assert!(is_inside_any((100, 100), &areas));
        assert!(is_inside_any((2000, 1000), &areas));
    }

    #[test]
    fn point_outside_every_work_area_is_rejected() {
        let areas = [(0, 0, 1920, 1040), (1920, 0, 2560, 1400)];
        assert!(!is_inside_any((100, 1040), &areas));
        assert!(!is_inside_any((-1, 0), &areas));
        assert!(!is_inside_any((5000, 100), &areas));
    }

    #[test]
    fn logical_size_converts_to_rounded_physical_size() {
        assert_eq!(physical_size((240.0, 80.0), 2.0), (480, 160));
        assert_eq!(physical_size((240.0, 44.0), 1.25), (300, 55));
    }

    #[test]
    fn pill_at_the_bottom_right_corner_is_clamped_inside_at_scale_two() {
        let size = physical_size((240.0, 80.0), 2.0);
        assert_eq!(
            clamp_origin((3800.0, 2100.0), (size.0 as f64, size.1 as f64), (0.0, 0.0, 3840.0, 2160.0)),
            (3360.0, 2000.0)
        );
    }

    #[test]
    fn a_drag_is_stored_only_when_the_widget_does_not_open_at_the_cursor() {
        assert!(!should_store_drag(true));
        assert!(should_store_drag(false));
    }

    #[test]
    fn default_origin_is_bottom_center_above_the_work_area_edge() {
        assert_eq!(default_origin((0, 0, 1920, 1040), (240, 44), 1.0), (840, 972));
        assert_eq!(default_origin((0, 0, 3840, 2080), (480, 88), 2.0), (1680, 1944));
    }
}
