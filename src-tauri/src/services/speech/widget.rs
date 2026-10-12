use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::time::Duration;

use serde::Serialize;
use tauri::{AppHandle, Emitter, Manager, PhysicalPosition, PhysicalSize};
use tokio::sync::Mutex;

use super::reminder::rms;
use super::{PendingPrompt, SpeechService, DICTATION_ID, DICTATION_LABEL};
use crate::commands::context::current_context_version;
use crate::models::context::ContextItem;
use crate::models::skill::{is_offered_in, OfferedIn, Skill};
use crate::services::config::ConfigService;
use crate::services::context::ContextManagerService;
use crate::services::monitor::{clamp_origin, find_monitor_at};
use crate::services::skill::SkillService;
use crate::services::ui_state::UiStateService;

pub const WINDOW_LABEL: &str = "recording-widget";
pub const WINDOW_POSITION_KEY: &str = "recording_widget.window_position";
pub const SHELL_POSITION_KEY: &str = "recording_widget.shell_position";

const STATE_EVENT: &str = "recording-widget-state";
const MODEL_EVENT: &str = "recording-widget-model";
const PREVIEW_FRAME_HEIGHT: f64 = 166.0;
const DROPDOWN_CLOSED_EVENT: &str = "recording-widget-dropdown-closed";
const TICK: Duration = Duration::from_millis(50);
const DONE_VISIBLE: Duration = Duration::from_secs(1);
const MOVE_DEBOUNCE: Duration = Duration::from_millis(500);
const FULL_SCALE_RMS: f64 = 32768.0;
const FLOOR_DB: f64 = -50.0;
const CEIL_DB: f64 = -20.0;
const LEVEL_ATTACK: f64 = 0.6;
const LEVEL_RELEASE: f64 = 0.2;
const BOTTOM_MARGIN: f64 = 24.0;
const PILL_WIDTH: f64 = 264.0;
const PILL_HEIGHT: f64 = 72.0;
const MAX_ITEM_ROWS: u32 = 3;
const ITEM_ROW_HEIGHT: f64 = 22.0;
const ITEM_ROW_GAP: f64 = 4.0;
const ITEMS_ROW_PADDING: f64 = 10.0;
const ITEM_COLUMNS: usize = 2;
const PREVIEW_CHARS: usize = 300;
const DROPDOWN_ROW_HEIGHT: f64 = 28.0;
const DROPDOWN_PADDING: f64 = 4.0;

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

#[derive(Debug, Clone, PartialEq)]
pub struct PillItem {
    pub index: u32,
    pub kind: String,
    pub label: String,
    pub preview: String,
}

#[derive(Debug, Clone, PartialEq)]
pub struct PillModel {
    pub targets: Vec<(String, String)>,
    pub selected_id: String,
    pub selected_label: String,
    pub target_locked: bool,
    pub items: Vec<PillItem>,
    pub item_rows: u32,
    pub context_version: u64,
    pub x: i32,
    pub y: i32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OverlayKind {
    Dropdown,
    Preview,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Overlay {
    pub kind: OverlayKind,
    pub up: bool,
    pub height: f64,
}

#[derive(Debug, Clone, Copy)]
pub enum OverlayRequest {
    Open(Overlay),
    Close(OverlayKind),
}

pub fn next_overlay(
    current: Option<Overlay>,
    request: OverlayRequest,
) -> (Option<Overlay>, Option<OverlayKind>) {
    match request {
        OverlayRequest::Open(opened) => {
            let replaced = current.map(|c| c.kind).filter(|kind| *kind != opened.kind);
            (Some(opened), replaced)
        }
        OverlayRequest::Close(kind) => match current {
            Some(c) if c.kind == kind => (None, Some(kind)),
            _ => (current, None),
        },
    }
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
    item_rows: u32,
    scale: f64,
    overlay: Option<Overlay>,
    #[cfg(target_os = "linux")]
    shell: Option<crate::services::gnome_shell::ShellProxy<'static>>,
}

impl Inner {
    fn is_current(&self, generation: u64) -> bool {
        self.transport.is_some() && self.generation == generation
    }
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

pub fn item_rows(count: usize) -> u32 {
    (count.div_ceil(ITEM_COLUMNS) as u32).min(MAX_ITEM_ROWS)
}

pub fn items_height(rows: u32) -> f64 {
    if rows == 0 {
        return 0.0;
    }
    rows as f64 * ITEM_ROW_HEIGHT + (rows - 1) as f64 * ITEM_ROW_GAP + ITEMS_ROW_PADDING
}

pub fn rows_resize(old_rows: u32, new_rows: u32) -> bool {
    old_rows != new_rows
}

pub fn pill_size(rows: u32) -> (f64, f64) {
    (PILL_WIDTH, PILL_HEIGHT + items_height(rows))
}

pub fn text_preview(content: &str) -> String {
    let mut chars = content.chars();
    let head: String = chars.by_ref().take(PREVIEW_CHARS).collect();
    if chars.next().is_some() {
        format!("{head}…")
    } else {
        head
    }
}

pub fn regrow_origin(
    origin: (f64, f64),
    old_height: f64,
    new_height: f64,
    width: f64,
    work_area: WorkArea,
) -> (f64, f64) {
    clamp_origin(
        (origin.0, origin.1 + old_height - new_height),
        (width, new_height),
        work_area,
    )
}

pub fn dropdown_opens_up(pill_origin_y: f64, pill_height: f64, work_area: WorkArea) -> bool {
    pill_origin_y + pill_height / 2.0 > work_area.1 + work_area.3 / 2.0
}

pub fn dropdown_height(rows: u32) -> f64 {
    rows as f64 * DROPDOWN_ROW_HEIGHT + 2.0 * DROPDOWN_PADDING
}

pub fn grown_frame(
    origin: (f64, f64),
    size: (f64, f64),
    list_height: f64,
    up: bool,
    work_area: WorkArea,
) -> ((f64, f64), (f64, f64)) {
    let grown = (size.0, size.1 + list_height);
    let top = if up { origin.1 - list_height } else { origin.1 };
    (clamp_origin((origin.0, top), grown, work_area), grown)
}

pub fn pill_targets(skills: &[Skill]) -> Vec<(String, String)> {
    let dictation = (DICTATION_ID.to_string(), DICTATION_LABEL.to_string());
    let offered = skills
        .iter()
        .filter(|skill| is_offered_in(&skill.offered_in, OfferedIn::Pill))
        .map(|skill| (skill.name.clone(), skill.display_name.clone()));
    std::iter::once(dictation).chain(offered).collect()
}

pub fn pill_items(items: &[ContextItem]) -> Vec<PillItem> {
    let (mut texts, mut images) = (0, 0);
    items
        .iter()
        .enumerate()
        .map(|(index, item)| {
            let (kind, label, preview) = match item {
                ContextItem::Text { content } => {
                    texts += 1;
                    ("text", format!("Text #{texts}"), text_preview(content))
                }
                ContextItem::Image { .. } => {
                    images += 1;
                    ("image", format!("Image #{images}"), String::new())
                }
            };
            PillItem { index: index as u32, kind: kind.to_string(), label, preview }
        })
        .collect()
}

pub fn build_model(
    targets: Vec<(String, String)>,
    pending: Option<&PendingPrompt>,
    target_locked: bool,
    context_items: &[ContextItem],
    context_version: u64,
    origin: (i32, i32),
) -> PillModel {
    let (selected_id, selected_label) = match pending {
        Some(pending) => (pending.id.clone(), pending.name.clone()),
        None => (DICTATION_ID.to_string(), DICTATION_LABEL.to_string()),
    };
    let items = pill_items(context_items);
    PillModel {
        targets,
        selected_id,
        selected_label,
        target_locked,
        item_rows: item_rows(items.len()),
        items,
        context_version,
        x: origin.0,
        y: origin.1,
    }
}

fn window_payload(model: &PillModel) -> serde_json::Value {
    serde_json::json!({
        "targets": model
            .targets
            .iter()
            .map(|(id, label)| serde_json::json!({ "id": id, "label": label }))
            .collect::<Vec<_>>(),
        "selectedId": model.selected_id,
        "selectedLabel": model.selected_label,
        "targetLocked": model.target_locked,
        "items": model
            .items
            .iter()
            .map(|item| {
                serde_json::json!({
                    "index": item.index,
                    "kind": item.kind,
                    "label": item.label,
                    "preview": item.preview,
                })
            })
            .collect::<Vec<_>>(),
        "itemRows": model.item_rows,
        "contextVersion": model.context_version,
        "x": model.x,
        "y": model.y,
    })
}

async fn gather_model(app: &AppHandle, origin: (i32, i32)) -> (PillModel, Vec<ContextItem>) {
    let targets = match app.try_state::<Arc<Mutex<SkillService>>>() {
        Some(skills) => pill_targets(skills.lock().await.list_skills()),
        None => pill_targets(&[]),
    };
    let items = match app.try_state::<Arc<Mutex<ContextManagerService>>>() {
        Some(context) => context.lock().await.get_items(),
        None => Vec::new(),
    };
    let speech = app.try_state::<Arc<Mutex<SpeechService>>>();
    let (pending, locked) = match &speech {
        Some(speech) => {
            let speech = speech.lock().await;
            (speech.shown_target().cloned(), speech.target_locked())
        }
        None => (None, false),
    };
    let model = build_model(targets, pending.as_ref(), locked, &items, current_context_version(), origin);
    (model, items)
}

fn placement_origin(inner: &Inner) -> (i32, i32) {
    inner
        .placement
        .map_or((0, 0), |p| (p.origin.0.round() as i32, p.origin.1.round() as i32))
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
    size: (f64, f64),
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
            size,
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
    log_placement(source, placement.origin, size, placement.work_area);
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
                item_rows: 0,
                scale: 1.0,
                overlay: None,
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

        let item_rows = gather_model(app, (0, 0)).await.0.item_rows;
        let mut inner = self.inner.lock().await;
        inner.generation += 1;
        inner.session = session;
        inner.item_rows = item_rows;
        inner.scale = 1.0;
        inner.state = WidgetState::Recording;
        inner.elapsed_ms = 0;
        inner.monochromatic = settings.monochromatic;
        inner.update_failed = false;
        inner.placement = None;
        inner.overlay = None;
        inner.transport = None;
        #[cfg(target_os = "linux")]
        {
            inner.shell = None;
        }

        let shown = match transport {
            #[cfg(target_os = "linux")]
            Transport::Shell => show_in_shell(app, settings, pill_size(item_rows)).await.map(|(proxy, placement)| {
                inner.shell = Some(proxy);
                inner.placement = Some(placement);
            }),
            Transport::Window => self.place_window(app, settings, item_rows).await.map(|(placement, scale)| {
                inner.placement = Some(placement);
                inner.scale = scale;
            }),
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
        push_current_model(app, &mut inner).await;
        if transport == Transport::Window {
            if let Err(e) = reveal_window(app) {
                log::warn!("showing the recording widget failed, using toasts: {e}");
                inner.transport = None;
                return false;
            }
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

    pub async fn generation(&self) -> u64 {
        self.inner.lock().await.generation
    }

    pub async fn done(&self, app: &AppHandle) {
        self.done_if(app, None).await;
    }

    pub async fn done_if_current(&self, app: &AppHandle, generation: u64) {
        self.done_if(app, Some(generation)).await;
    }

    pub async fn close_if_current(&self, app: &AppHandle, generation: u64) {
        let mut inner = self.inner.lock().await;
        if inner.is_current(generation) {
            self.close_locked(app, &mut inner).await;
        } else {
            log::debug!("recording widget: close skipped, stale generation={generation}");
        }
    }

    async fn done_if(&self, app: &AppHandle, only_generation: Option<u64>) {
        let generation = {
            let mut inner = self.inner.lock().await;
            let skip = match only_generation {
                Some(g) => !inner.is_current(g),
                None => inner.transport.is_none(),
            };
            if skip {
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
        if inner.overlay.take().is_some() {
            let _ = app.emit_to(WINDOW_LABEL, DROPDOWN_CLOSED_EVENT, ());
        }
        let result = match transport {
            #[cfg(target_os = "linux")]
            Transport::Shell => {
                clear_preview_files(app);
                match inner.shell.take() {
                    Some(proxy) => proxy.hide_recording_widget().await.map_err(|e| e.to_string()),
                    None => Ok(()),
                }
            }
            Transport::Window => match app.get_webview_window(WINDOW_LABEL) {
                Some(window) => window.hide().map_err(|e| e.to_string()),
                None => Ok(()),
            },
        };
        if let Err(e) = result {
            log::warn!("hiding the recording widget failed: {e}");
        }
    }

    async fn place_window(
        &self,
        app: &AppHandle,
        settings: WidgetSettings,
        item_rows: u32,
    ) -> Result<(Placement, f64), String> {
        let window = app
            .get_webview_window(WINDOW_LABEL)
            .ok_or("recording-widget window not found")?;
        let logical_size = pill_size(item_rows);
        let (origin, monitor, source) = if settings.at_cursor {
            let (cx, cy) = app
                .cursor_position()
                .map(|p| (p.x, p.y))
                .unwrap_or((0.0, 0.0));
            let monitor = find_monitor_at(app, cx as i32, cy as i32)?;
            ((cx, cy), monitor, "cursor")
        } else {
            let areas = work_areas(app).await?;
            let stored = read_position(app, WINDOW_POSITION_KEY)
                .await
                .filter(|&point| is_inside_any(point, &areas));
            match stored {
                Some((x, y)) => {
                    let monitor = find_monitor_at(app, x, y)?;
                    ((x as f64, y as f64), monitor, "stored position")
                }
                None => {
                    let (cx, cy) = app
                        .cursor_position()
                        .map(|p| (p.x as i32, p.y as i32))
                        .unwrap_or((0, 0));
                    let monitor = find_monitor_at(app, cx, cy)?;
                    let work = monitor.work_area();
                    let size = physical_size(logical_size, monitor.scale_factor());
                    let (x, y) = default_origin(
                        (work.position.x, work.position.y, work.size.width, work.size.height),
                        size,
                        monitor.scale_factor(),
                    );
                    ((x as f64, y as f64), monitor, "default")
                }
            }
        };
        let scale = monitor.scale_factor();
        let work_area = work_area_of(&monitor);
        let size = physical_size(logical_size, scale);
        window
            .set_size(PhysicalSize::new(size.0, size.1))
            .map_err(|e| e.to_string())?;
        let origin = clamp_origin(origin, (size.0 as f64, size.1 as f64), work_area);
        let origin = (origin.0.round(), origin.1.round());
        log_placement(source, origin, (size.0 as f64, size.1 as f64), work_area);
        self.set_window_origin(&window, origin)?;
        Ok((Placement { origin, work_area }, scale))
    }

    fn mark_programmatic(&self, origin: (f64, f64)) {
        *self
            .programmatic_origin
            .lock()
            .unwrap_or_else(|e| e.into_inner()) = Some((origin.0 as i32, origin.1 as i32));
    }

    fn set_window_origin(&self, window: &tauri::WebviewWindow, origin: (f64, f64)) -> Result<(), String> {
        self.mark_programmatic(origin);
        window
            .set_position(PhysicalPosition::new(origin.0 as i32, origin.1 as i32))
            .map_err(|e| e.to_string())
    }

    pub async fn refresh_model(&self, app: &AppHandle) {
        let mut inner = self.inner.lock().await;
        if inner.transport.is_none() {
            return;
        }
        let (model, context_items) = gather_model(app, placement_origin(&inner)).await;
        if rows_resize(inner.item_rows, model.item_rows) {
            self.resize_locked(app, &mut inner, model.item_rows);
        }
        push_model(app, &mut inner, model, &context_items).await;
    }

    fn resize_locked(&self, app: &AppHandle, inner: &mut Inner, item_rows: u32) {
        if inner.overlay.take().is_some() {
            let _ = app.emit_to(WINDOW_LABEL, DROPDOWN_CLOSED_EVENT, ());
        }
        let scale = inner.scale;
        let (width, old_height) = pill_size(inner.item_rows);
        let new_height = pill_size(item_rows).1;
        let old_rows = std::mem::replace(&mut inner.item_rows, item_rows);
        let Some(placement) = inner.placement.as_mut() else {
            return;
        };
        placement.origin = regrow_origin(
            placement.origin,
            old_height * scale,
            new_height * scale,
            width * scale,
            placement.work_area,
        );
        log::debug!(
            "recording widget: resized rows {old_rows} -> {item_rows} origin={:?}",
            placement.origin
        );
        if inner.transport != Some(Transport::Window) {
            return;
        }
        let origin = placement.origin;
        let size = physical_size((width, new_height), scale);
        self.apply_frame(app, origin, (size.0 as f64, size.1 as f64), scale);
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
            let mut stored = position;
            if let Ok(monitor) = find_monitor_at(&app, position.0, position.1) {
                let work_area = work_area_of(&monitor);
                let mut inner = widget.inner.lock().await;
                inner.scale = monitor.scale_factor();
                let origin = match inner.overlay.take() {
                    Some(overlay) => {
                        let top = if overlay.up {
                            position.1 as f64 + overlay.height
                        } else {
                            position.1 as f64
                        };
                        let size = physical_size(pill_size(inner.item_rows), inner.scale);
                        let origin = clamp_origin(
                            (position.0 as f64, top),
                            (size.0 as f64, size.1 as f64),
                            work_area,
                        );
                        let origin = (origin.0.round(), origin.1.round());
                        log::debug!("recording widget: overlay closed by drag origin={origin:?} size={size:?}");
                        widget.apply_frame(&app, origin, (size.0 as f64, size.1 as f64), inner.scale);
                        let _ = app.emit_to(WINDOW_LABEL, DROPDOWN_CLOSED_EVENT, ());
                        origin
                    }
                    None => (position.0 as f64, position.1 as f64),
                };
                inner.placement = Some(Placement { origin, work_area });
                stored = (origin.0 as i32, origin.1 as i32);
            }
            widget.store_drag_debounced(&app, WINDOW_POSITION_KEY, stored).await;
        });
    }

    fn apply_frame(&self, app: &AppHandle, origin: (f64, f64), size: (f64, f64), scale: f64) {
        let Some(window) = app.get_webview_window(WINDOW_LABEL) else {
            log::warn!("recording widget frame skipped: window not found");
            return;
        };
        #[cfg(target_os = "macos")]
        let applied = {
            self.mark_programmatic(origin);
            let primary_height_points = app
                .primary_monitor()
                .ok()
                .flatten()
                .map_or(0.0, |m| m.size().height as f64 / m.scale_factor());
            crate::services::macos_panel::set_frame_physical(&window, origin, size, scale, primary_height_points)
        };
        #[cfg(not(target_os = "macos"))]
        let applied = {
            let _ = scale;
            window
                .set_size(PhysicalSize::new(size.0 as u32, size.1 as u32))
                .map_err(|e| e.to_string())
                .and_then(|_| self.set_window_origin(&window, origin))
        };
        if let Err(e) = applied {
            log::warn!("setting the recording widget frame failed: {e}");
        }
    }

    pub async fn open_dropdown(&self, app: &AppHandle, rows: u32) -> Result<bool, String> {
        self.open_overlay(app, OverlayKind::Dropdown, dropdown_height(rows)).await
    }

    pub async fn open_preview(&self, app: &AppHandle) -> Result<bool, String> {
        self.open_overlay(app, OverlayKind::Preview, PREVIEW_FRAME_HEIGHT).await
    }

    pub async fn close_dropdown(&self, app: &AppHandle) {
        self.close_overlay(app, OverlayKind::Dropdown).await;
    }

    pub async fn close_preview(&self, app: &AppHandle) {
        self.close_overlay(app, OverlayKind::Preview).await;
    }

    async fn open_overlay(
        &self,
        app: &AppHandle,
        kind: OverlayKind,
        logical_height: f64,
    ) -> Result<bool, String> {
        let mut inner = self.inner.lock().await;
        let placement = match (inner.transport, inner.placement) {
            (Some(Transport::Window), Some(placement)) => placement,
            _ => return Err("recording widget window is not shown".into()),
        };
        let scale = inner.scale;
        let size = physical_size(pill_size(inner.item_rows), scale);
        let pill = (size.0 as f64, size.1 as f64);
        let up = dropdown_opens_up(placement.origin.1, pill.1, placement.work_area);
        let height = logical_height * scale;
        let (origin, grown) = grown_frame(placement.origin, pill, height, up, placement.work_area);
        let origin = (origin.0.round(), origin.1.round());
        log::debug!("recording widget: {kind:?} open up={up} origin={origin:?} size={grown:?}");
        self.apply_frame(app, origin, grown, scale);
        let request = OverlayRequest::Open(Overlay { kind, up, height });
        inner.overlay = next_overlay(inner.overlay, request).0;
        Ok(up)
    }

    async fn close_overlay(&self, app: &AppHandle, kind: OverlayKind) {
        let mut inner = self.inner.lock().await;
        let (overlay, closed) = next_overlay(inner.overlay, OverlayRequest::Close(kind));
        if closed.is_none() {
            return;
        }
        inner.overlay = overlay;
        let scale = inner.scale;
        let size = physical_size(pill_size(inner.item_rows), scale);
        let size = (size.0 as f64, size.1 as f64);
        if let Some(placement) = inner.placement.as_mut() {
            let origin = clamp_origin(placement.origin, size, placement.work_area);
            placement.origin = (origin.0.round(), origin.1.round());
            log::debug!("recording widget: {kind:?} close origin={:?} size={size:?}", placement.origin);
            self.apply_frame(app, placement.origin, size, scale);
        }
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

fn note_update_failure(inner: &mut Inner, result: Result<(), String>) {
    if let Err(e) = result {
        if !inner.update_failed {
            inner.update_failed = true;
            log::warn!("updating the recording widget failed: {e}");
        }
    }
}

fn reveal_window(app: &AppHandle) -> Result<(), String> {
    app.get_webview_window(WINDOW_LABEL)
        .ok_or("recording-widget window not found")?
        .show()
        .map_err(|e| e.to_string())
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
    note_update_failure(inner, result);
}

async fn push_current_model(app: &AppHandle, inner: &mut Inner) {
    let (model, context_items) = gather_model(app, placement_origin(inner)).await;
    push_model(app, inner, model, &context_items).await;
}

#[cfg(target_os = "linux")]
fn preview_dir(app: &AppHandle) -> Result<std::path::PathBuf, String> {
    app.path()
        .app_cache_dir()
        .map(|dir| dir.join(super::pill_previews::DIR_NAME))
        .map_err(|e| e.to_string())
}

#[cfg(target_os = "linux")]
fn clear_preview_files(app: &AppHandle) {
    match preview_dir(app) {
        Ok(dir) => {
            tauri::async_runtime::spawn_blocking(move || super::pill_previews::clear_previews(&dir));
        }
        Err(e) => log::warn!("resolving the pill preview directory failed: {e}"),
    }
}

#[cfg(target_os = "linux")]
async fn sync_preview_files(app: &AppHandle, context_items: &[ContextItem]) -> Vec<String> {
    let images: Vec<(String, String)> = context_items
        .iter()
        .filter_map(|item| match item {
            ContextItem::Image { data, media_type } => Some((data.clone(), media_type.clone())),
            ContextItem::Text { .. } => None,
        })
        .collect();
    let failed = vec![String::new(); images.len()];
    let dir = match preview_dir(app) {
        Ok(dir) => dir,
        Err(e) => {
            log::warn!("resolving the pill preview directory failed: {e}");
            return failed;
        }
    };
    let synced = {
        let dir = dir.clone();
        tauri::async_runtime::spawn_blocking(move || super::pill_previews::sync_previews(&dir, &images)).await
    };
    match synced {
        Ok(Ok(paths)) => {
            log::debug!("recording widget: synced {} preview files in {}", paths.len(), dir.display());
            paths.iter().map(|path| path.to_string_lossy().into_owned()).collect()
        }
        Ok(Err(e)) => {
            log::warn!("writing the pill previews in {} failed: {e}", dir.display());
            failed
        }
        Err(e) => {
            log::warn!("the pill preview task for {} failed: {e}", dir.display());
            failed
        }
    }
}

async fn push_model(app: &AppHandle, inner: &mut Inner, mut model: PillModel, context_items: &[ContextItem]) {
    let Some(transport) = inner.transport else {
        return;
    };
    #[cfg(not(target_os = "linux"))]
    let _ = context_items;
    if let Some(placement) = inner.placement {
        model.x = placement.origin.0.round() as i32;
        model.y = placement.origin.1.round() as i32;
    }
    let result = match transport {
        #[cfg(target_os = "linux")]
        Transport::Shell => match &inner.shell {
            Some(proxy) => {
                let mut paths = sync_preview_files(app, context_items).await.into_iter();
                let items = model
                    .items
                    .into_iter()
                    .zip(context_items)
                    .map(|(item, source)| {
                        let image_path = match source {
                            ContextItem::Image { .. } => paths.next().unwrap_or_default(),
                            ContextItem::Text { .. } => String::new(),
                        };
                        (item.index, item.kind, item.label, item.preview, image_path)
                    })
                    .collect();
                proxy
                    .update_recording_widget_model(
                        model.targets,
                        &model.selected_id,
                        &model.selected_label,
                        model.target_locked,
                        items,
                        model.item_rows,
                        model.context_version,
                        model.x,
                        model.y,
                    )
                    .await
                    .map_err(|e| e.to_string())
            }
            None => Ok(()),
        },
        Transport::Window => app
            .emit_to(WINDOW_LABEL, MODEL_EVENT, window_payload(&model))
            .map_err(|e| e.to_string()),
    };
    note_update_failure(inner, result);
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

    fn inner_with(transport: Option<Transport>, generation: u64, state: WidgetState) -> Inner {
        Inner {
            transport,
            session: 1,
            state,
            generation,
            elapsed_ms: 0,
            monochromatic: true,
            update_failed: false,
            placement: None,
            item_rows: 0,
            scale: 1.0,
            overlay: None,
            #[cfg(target_os = "linux")]
            shell: None,
        }
    }

    #[test]
    fn a_stale_generation_is_not_current_and_leaves_a_newer_pill_unchanged() {
        let newer = inner_with(Some(Transport::Window), 5, WidgetState::Recording);
        assert!(!newer.is_current(4));
        assert!(newer.is_current(5));
        assert_eq!(newer.state, WidgetState::Recording);
        assert_eq!(newer.generation, 5);
    }

    #[test]
    fn a_closed_pill_is_not_current() {
        let closed = inner_with(None, 5, WidgetState::Processing);
        assert!(!closed.is_current(5));
    }

    fn skill(name: &str, display_name: &str, offered_in: Option<Vec<OfferedIn>>) -> Skill {
        Skill {
            name: name.into(),
            display_name: display_name.into(),
            description: None,
            model: None,
            parameters: None,
            offered_in,
            body: String::new(),
            file_path: Default::default(),
            skill_version_id: None,
        }
    }

    fn text() -> ContextItem {
        ContextItem::Text { content: "t".into() }
    }

    fn image() -> ContextItem {
        ContextItem::Image { data: "d".into(), media_type: "image/png".into() }
    }

    #[test]
    fn item_rows_add_their_height_to_the_pill() {
        assert_eq!(pill_size(0), (264.0, 72.0));
        assert_eq!(pill_size(1), (264.0, 104.0));
        assert_eq!(pill_size(2), (264.0, 130.0));
        assert_eq!(pill_size(3), (264.0, 156.0));
    }

    #[test]
    fn chips_fill_two_columns_up_to_three_rows() {
        assert_eq!(item_rows(0), 0);
        assert_eq!(item_rows(1), 1);
        assert_eq!(item_rows(2), 1);
        assert_eq!(item_rows(3), 2);
        assert_eq!(item_rows(7), 3);
    }

    #[test]
    fn only_a_row_change_resizes() {
        assert!(!rows_resize(2, 2));
        assert!(rows_resize(2, 1));
    }

    #[test]
    fn a_long_text_preview_is_cut_on_chars() {
        let preview = text_preview(&"ą".repeat(400));
        assert_eq!(preview, format!("{}…", "ą".repeat(300)));
        assert_eq!(text_preview("a\nb"), "a\nb");
    }

    #[test]
    fn an_image_item_has_no_preview() {
        assert_eq!(pill_items(&[image()])[0].preview, "");
    }

    #[test]
    fn regrow_keeps_the_bottom_edge() {
        assert_eq!(
            regrow_origin((100.0, 500.0), 80.0, 112.0, 240.0, (0.0, 0.0, 1920.0, 1040.0)),
            (100.0, 468.0)
        );
    }

    #[test]
    fn regrow_from_one_to_three_rows_moves_up_by_the_difference() {
        assert_eq!(
            regrow_origin((100.0, 500.0), 112.0, 164.0, 240.0, (0.0, 0.0, 1920.0, 1040.0)),
            (100.0, 448.0)
        );
    }

    #[test]
    fn regrow_is_clamped_to_the_work_area_top() {
        assert_eq!(
            regrow_origin((100.0, 10.0), 80.0, 112.0, 240.0, (0.0, 0.0, 1920.0, 1040.0)),
            (100.0, 0.0)
        );
    }

    const WORK_AREA: WorkArea = (0.0, 0.0, 3840.0, 2160.0);

    #[test]
    fn a_pill_in_the_upper_half_opens_the_dropdown_down() {
        assert!(!dropdown_opens_up(900.0, 160.0, WORK_AREA));
    }

    #[test]
    fn a_pill_in_the_lower_half_opens_the_dropdown_up() {
        assert!(dropdown_opens_up(1800.0, 160.0, WORK_AREA));
    }

    #[test]
    fn the_dropdown_grows_upward_keeping_the_bottom_edge() {
        assert_eq!(
            grown_frame((200.0, 1800.0), (480.0, 160.0), 240.0, true, WORK_AREA),
            ((200.0, 1560.0), (480.0, 400.0))
        );
    }

    #[test]
    fn the_dropdown_grows_downward_keeping_the_top_edge() {
        assert_eq!(
            grown_frame((200.0, 100.0), (480.0, 160.0), 240.0, false, WORK_AREA),
            ((200.0, 100.0), (480.0, 400.0))
        );
    }

    #[test]
    fn the_preview_opens_up_from_a_pill_in_the_lower_half() {
        assert_eq!(
            grown_frame((100.0, 900.0), (240.0, 112.0), 166.0, true, (0.0, 0.0, 1920.0, 1040.0)),
            ((100.0, 734.0), (240.0, 278.0))
        );
    }

    #[test]
    fn the_preview_opens_down_from_a_pill_in_the_upper_half() {
        assert_eq!(
            grown_frame((100.0, 100.0), (240.0, 112.0), 166.0, false, (0.0, 0.0, 1920.0, 1040.0)),
            ((100.0, 100.0), (240.0, 278.0))
        );
    }

    fn overlay(kind: OverlayKind) -> Overlay {
        Overlay { kind, up: false, height: 10.0 }
    }

    #[test]
    fn opening_a_preview_replaces_the_dropdown_and_reports_it_closed() {
        let current = Some(overlay(OverlayKind::Dropdown));
        let request = OverlayRequest::Open(overlay(OverlayKind::Preview));
        assert_eq!(
            next_overlay(current, request),
            (Some(overlay(OverlayKind::Preview)), Some(OverlayKind::Dropdown))
        );
    }

    #[test]
    fn opening_a_dropdown_replaces_the_preview_and_reports_it_closed() {
        let current = Some(overlay(OverlayKind::Preview));
        let request = OverlayRequest::Open(overlay(OverlayKind::Dropdown));
        assert_eq!(
            next_overlay(current, request),
            (Some(overlay(OverlayKind::Dropdown)), Some(OverlayKind::Preview))
        );
    }

    #[test]
    fn closing_a_preview_leaves_an_open_dropdown_unchanged() {
        let current = Some(overlay(OverlayKind::Dropdown));
        assert_eq!(
            next_overlay(current, OverlayRequest::Close(OverlayKind::Preview)),
            (current, None)
        );
    }

    #[test]
    fn targets_start_with_plain_dictation() {
        let targets = pill_targets(&[skill("fix", "Fix", None)]);
        assert_eq!(targets[0], ("__dictation__".to_string(), "Dictation".to_string()));
        assert_eq!(targets[1], ("fix".to_string(), "Fix".to_string()));
    }

    #[test]
    fn a_context_menu_only_skill_is_not_a_target() {
        let targets = pill_targets(&[skill("fix", "Fix", Some(vec![OfferedIn::ContextMenu]))]);
        assert_eq!(targets.len(), 1);
    }

    #[test]
    fn a_skill_without_offered_in_is_a_target() {
        assert_eq!(pill_targets(&[skill("fix", "Fix", None)]).len(), 2);
    }

    #[test]
    fn targets_follow_the_skill_order() {
        let targets = pill_targets(&[
            skill("b", "B", Some(vec![OfferedIn::Pill])),
            skill("a", "A", None),
        ]);
        let ids: Vec<&str> = targets.iter().map(|(id, _)| id.as_str()).collect();
        assert_eq!(ids, ["__dictation__", "b", "a"]);
    }

    #[test]
    fn items_are_labelled_per_kind_in_context_order() {
        let items = pill_items(&[text(), image(), text()]);
        let labels: Vec<&str> = items.iter().map(|item| item.label.as_str()).collect();
        assert_eq!(labels, ["Text #1", "Image #1", "Text #2"]);
        assert_eq!(
            items[1],
            PillItem { index: 1, kind: "image".into(), label: "Image #1".into(), preview: String::new() }
        );
    }

    #[test]
    fn a_shift_click_chat_is_selected_even_when_not_a_target() {
        let pending = PendingPrompt {
            id: "__chat__".into(),
            name: "Chat".into(),
            origin: crate::services::speech::PendingOrigin::ShiftClick,
        };
        let model = build_model(pill_targets(&[]), Some(&pending), false, &[], 1, (0, 0));
        assert_eq!(model.selected_label, "Chat");
        assert!(!model.targets.iter().any(|(id, _)| id == &model.selected_id));
    }

    #[test]
    fn no_pending_selects_plain_dictation() {
        let model = build_model(pill_targets(&[]), None, false, &[text()], 7, (3, 4));
        assert_eq!((model.selected_id.as_str(), model.selected_label.as_str()), ("__dictation__", "Dictation"));
        assert_eq!((model.context_version, model.x, model.y), (7, 3, 4));
        assert_eq!(model.items.len(), 1);
    }

    #[test]
    fn the_model_is_locked_only_after_the_take() {
        let mut speech = SpeechService::new();
        let model = |speech: &SpeechService| {
            build_model(vec![], speech.shown_target(), speech.target_locked(), &[], 0, (0, 0))
        };
        assert!(!model(&speech).target_locked);
        speech.take_pending_prompt();
        assert!(model(&speech).target_locked);
    }

    #[test]
    fn the_taken_skill_stays_on_the_locked_pill() {
        let mut speech = SpeechService::new();
        speech.select_target("assistant", "Assistant").unwrap();
        speech.take_pending_prompt();
        let model = build_model(vec![], speech.shown_target(), speech.target_locked(), &[], 0, (0, 0));
        assert_eq!((model.selected_id.as_str(), model.selected_label.as_str()), ("assistant", "Assistant"));
        assert!(model.target_locked);
        speech.set_pending_prompt("fix".into(), "Fix".into(), crate::services::speech::PendingOrigin::ShiftClick);
        assert_eq!(speech.shown_target().map(|p| p.id.as_str()), Some("assistant"));
    }

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
