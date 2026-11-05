use std::ffi::c_void;
use std::ptr;
use std::sync::Arc;
use std::time::{Duration, Instant};

use objc2_app_kit::{NSPasteboard, NSPasteboardTypeString, NSWorkspace};
use objc2_core_graphics::{
    CGEvent, CGEventFlags, CGEventSource, CGEventSourceStateID, CGEventTapLocation, CGKeyCode, CGMouseButton,
};
use objc2_foundation::NSString;
use tauri::{AppHandle, Manager};
use tokio::sync::Mutex;

use super::{
    equals_baseline, is_current, should_restore, AxSelection, Rect, Sample, TriggerDetector, COPY_WAIT_MS,
    POLL_INTERVAL_MS, SETTLE_DELAY_MS,
};
use crate::services::notification::{NotificationLevel, NotificationService};
use crate::services::speech::claude_desktop::macos::{
    accessibility_trusted_or_prompt, on_main, restore_pasteboard, save_pasteboard,
};
use crate::services::speech::screenshot::keep_capture;
use crate::services::speech::widget::{RecordingWidget, WINDOW_LABEL};
use crate::services::speech::SpeechService;

const POLL_INTERVAL: Duration = Duration::from_millis(POLL_INTERVAL_MS);
const SETTLE_DELAY: Duration = Duration::from_millis(SETTLE_DELAY_MS);
const COPY_WAIT: Duration = Duration::from_millis(COPY_WAIT_MS);
const ARROW_KEYS: [CGKeyCode; 4] = [123, 124, 125, 126];
const KEY_A: CGKeyCode = 0;
const KEY_C: CGKeyCode = 8;
const AX_VALUE_CF_RANGE: u32 = 4;
const CF_NUMBER_SINT64: isize = 4;

type CFTypeRef = *const c_void;

#[repr(C)]
struct CFRange {
    location: isize,
    length: isize,
}

#[link(name = "ApplicationServices", kind = "framework")]
extern "C" {
    fn AXUIElementCreateSystemWide() -> CFTypeRef;
    fn AXUIElementCreateApplication(pid: i32) -> CFTypeRef;
    fn AXUIElementCopyAttributeValue(element: CFTypeRef, attribute: &NSString, value: *mut CFTypeRef) -> i32;
    fn AXUIElementCopyParameterizedAttributeValue(
        element: CFTypeRef,
        attribute: &NSString,
        parameter: CFTypeRef,
        result: *mut CFTypeRef,
    ) -> i32;
    fn AXUIElementPerformAction(element: CFTypeRef, action: &NSString) -> i32;
    fn AXValueGetValue(value: CFTypeRef, kind: u32, out: *mut c_void) -> u8;
}

#[link(name = "CoreFoundation", kind = "framework")]
extern "C" {
    fn CFRelease(cf: CFTypeRef);
    fn CFRetain(cf: CFTypeRef) -> CFTypeRef;
    fn CFEqual(a: CFTypeRef, b: CFTypeRef) -> u8;
    fn CFGetTypeID(cf: CFTypeRef) -> usize;
    fn CFStringGetTypeID() -> usize;
    fn CFBooleanGetTypeID() -> usize;
    fn CFArrayGetCount(array: CFTypeRef) -> isize;
    fn CFArrayGetValueAtIndex(array: CFTypeRef, index: isize) -> CFTypeRef;
    fn CFBooleanGetValue(boolean: CFTypeRef) -> u8;
    fn CFNumberGetValue(number: CFTypeRef, kind: isize, out: *mut c_void) -> u8;
}

struct Cf(CFTypeRef);

unsafe impl Send for Cf {}

impl Cf {
    fn owned(raw: CFTypeRef) -> Option<Self> {
        (!raw.is_null()).then_some(Self(raw))
    }

    fn retained(borrowed: CFTypeRef) -> Option<Self> {
        (!borrowed.is_null()).then(|| Self(unsafe { CFRetain(borrowed) }))
    }

    fn type_id(&self) -> usize {
        unsafe { CFGetTypeID(self.0) }
    }

    fn attribute(&self, name: &str) -> Option<Cf> {
        let mut value = ptr::null();
        let status = unsafe { AXUIElementCopyAttributeValue(self.0, &NSString::from_str(name), &mut value) };
        if status != 0 {
            return None;
        }
        Cf::owned(value)
    }

    fn string(&self) -> Option<String> {
        (self.type_id() == unsafe { CFStringGetTypeID() })
            .then(|| unsafe { &*(self.0 as *const NSString) }.to_string())
    }

    fn string_attribute(&self, name: &str) -> Option<String> {
        self.attribute(name)?.string()
    }

    fn bool_attribute(&self, name: &str) -> Option<bool> {
        let value = self.attribute(name)?;
        (value.type_id() == unsafe { CFBooleanGetTypeID() }).then(|| unsafe { CFBooleanGetValue(value.0) } != 0)
    }

    fn number_attribute(&self, name: &str) -> Option<i64> {
        let value = self.attribute(name)?;
        let mut number = 0i64;
        let ok = unsafe { CFNumberGetValue(value.0, CF_NUMBER_SINT64, ptr::from_mut(&mut number).cast()) };
        (ok != 0).then_some(number)
    }

    fn range_attribute(&self, name: &str) -> Option<(isize, isize)> {
        let value = self.attribute(name)?;
        let mut range = CFRange { location: 0, length: 0 };
        let ok = unsafe { AXValueGetValue(value.0, AX_VALUE_CF_RANGE, ptr::from_mut(&mut range).cast()) };
        (ok != 0).then_some((range.location, range.length))
    }

    fn children(&self) -> Vec<Cf> {
        let Some(array) = self.attribute("AXChildren") else {
            return Vec::new();
        };
        let count = unsafe { CFArrayGetCount(array.0) };
        (0..count)
            .filter_map(|index| Cf::retained(unsafe { CFArrayGetValueAtIndex(array.0, index) }))
            .collect()
    }
}

impl Drop for Cf {
    fn drop(&mut self) {
        unsafe { CFRelease(self.0) };
    }
}

impl PartialEq for Cf {
    fn eq(&self, other: &Self) -> bool {
        unsafe { CFEqual(self.0, other.0) != 0 }
    }
}

fn marker_range_text(element: &Cf) -> Option<String> {
    let marker = element.attribute("AXSelectedTextMarkerRange")?;
    let mut text = ptr::null();
    let status = unsafe {
        AXUIElementCopyParameterizedAttributeValue(
            element.0,
            &NSString::from_str("AXStringForTextMarkerRange"),
            marker.0,
            &mut text,
        )
    };
    if status != 0 {
        return None;
    }
    Cf::owned(text)?.string()
}

fn read_ax_selection() -> Option<AxSelection<Cf>> {
    let system = Cf::owned(unsafe { AXUIElementCreateSystemWide() })?;
    let element = system.attribute("AXFocusedUIElement")?;
    let range = element.range_attribute("AXSelectedTextRange");
    let text = element
        .string_attribute("AXSelectedText")
        .filter(|text| !text.is_empty())
        .or_else(|| marker_range_text(&element))
        .unwrap_or_default();
    Some(AxSelection { element, range, text })
}

enum CopyItem {
    Missing,
    Disabled,
    Enabled(Cf),
}

fn is_copy_item(item: &Cf) -> bool {
    item.string_attribute("AXMenuItemCmdChar").as_deref() == Some("C")
        && item.number_attribute("AXMenuItemCmdModifiers") == Some(0)
}

fn find_copy_item(node: &Cf) -> Option<Cf> {
    node.children()
        .into_iter()
        .find_map(|child| if is_copy_item(&child) { Some(child) } else { find_copy_item(&child) })
}

fn frontmost_copy_item() -> CopyItem {
    let Some(app) = NSWorkspace::sharedWorkspace().frontmostApplication() else {
        return CopyItem::Missing;
    };
    let Some(application) = Cf::owned(unsafe { AXUIElementCreateApplication(app.processIdentifier()) }) else {
        return CopyItem::Missing;
    };
    let Some(item) = application
        .attribute("AXMenuBar")
        .and_then(|bar| find_copy_item(&bar))
    else {
        return CopyItem::Missing;
    };
    if item.bool_attribute("AXEnabled") == Some(false) {
        CopyItem::Disabled
    } else {
        CopyItem::Enabled(item)
    }
}

fn press_menu_item(item: &Cf) -> bool {
    unsafe { AXUIElementPerformAction(item.0, &NSString::from_str("AXPress")) == 0 }
}

fn post_copy_key() {
    let source = CGEventSource::new(CGEventSourceStateID::HIDSystemState);
    for down in [true, false] {
        let Some(event) = CGEvent::new_keyboard_event(source.as_deref(), KEY_C, down) else {
            log::warn!("selection capture: creating the Cmd+C event failed");
            return;
        };
        CGEvent::set_flags(Some(&event), CGEventFlags::MaskCommand);
        CGEvent::post(CGEventTapLocation::HIDEventTap, Some(&event));
    }
}

fn change_count() -> isize {
    NSPasteboard::generalPasteboard().changeCount()
}

fn read_sample(started: Instant) -> Option<Sample> {
    let source = CGEventSourceStateID::CombinedSessionState;
    let flags = CGEventSource::flags_state(source);
    let pointer = CGEvent::location(Some(&*CGEvent::new(None)?));
    Some(Sample {
        at_ms: started.elapsed().as_millis() as u64,
        left_down: CGEventSource::button_state(source, CGMouseButton::Left),
        x: pointer.x,
        y: pointer.y,
        shift: flags.contains(CGEventFlags::MaskShift),
        arrow: ARROW_KEYS.iter().any(|key| CGEventSource::key_state(source, *key)),
        command: flags.contains(CGEventFlags::MaskCommand),
        a_key: CGEventSource::key_state(source, KEY_A),
    })
}

fn pill_frame(app: &AppHandle) -> Rect {
    let frame = app.get_webview_window(WINDOW_LABEL).and_then(|window| {
        let scale = window.scale_factor().ok()?;
        let position = window.outer_position().ok()?;
        let size = window.outer_size().ok()?;
        Some(Rect {
            x: position.x as f64 / scale,
            y: position.y as f64 / scale,
            width: size.width as f64 / scale,
            height: size.height as f64 / scale,
        })
    });
    frame.unwrap_or_default()
}

async fn keep_running(app: &AppHandle, generation: u64, run: u64) -> bool {
    if !is_current(run) {
        return false;
    }
    let current = app
        .state::<RecordingWidget>()
        .shown_generation()
        .await
        .map(|(_, generation)| generation);
    let locked = app.state::<Arc<Mutex<SpeechService>>>().lock().await.target_locked();
    keep_capture(generation, current, locked)
}

async fn warn_permission_missing(app: &AppHandle) {
    log::info!("selection capture skipped: Accessibility permission missing");
    NotificationService::notify_now(
        app,
        "selection_permission",
        NotificationLevel::Warning,
        "Accessibility permission needed",
        Some("Allow Promptheus in System Settings → Privacy & Security → Accessibility, then try again"),
    )
    .await;
}

async fn wait_for_pasteboard_change(app: &AppHandle, before: isize) -> Option<isize> {
    let started = Instant::now();
    loop {
        let count = on_main(app, change_count).await.ok()?;
        if count != before {
            return Some(count);
        }
        if started.elapsed() >= COPY_WAIT {
            return None;
        }
        tokio::time::sleep(POLL_INTERVAL).await;
    }
}

async fn copy_fallback(app: &AppHandle, generation: u64, run: u64) -> Option<String> {
    let started = Instant::now();
    let item = match tauri::async_runtime::spawn_blocking(frontmost_copy_item).await.ok()? {
        CopyItem::Disabled => {
            log::debug!("selection capture: Copy menu item disabled, no selection");
            return None;
        }
        CopyItem::Missing => None,
        CopyItem::Enabled(item) => Some(item),
    };
    if !keep_running(app, generation, run).await {
        return None;
    }
    let (saved, before) = on_main(app, || (save_pasteboard(), change_count())).await.ok()?;
    let pressed = item.is_some();
    tauri::async_runtime::spawn_blocking(move || {
        if !item.is_some_and(|item| press_menu_item(&item)) {
            post_copy_key();
        }
    })
    .await
    .ok()?;
    let Some(after_copy) = wait_for_pasteboard_change(app, before).await else {
        log::debug!("selection capture: copy fallback changed nothing, menu item={pressed}");
        return None;
    };
    let text = on_main(app, move || {
        let text = NSPasteboard::generalPasteboard()
            .stringForType(unsafe { NSPasteboardTypeString })
            .map(|text| text.to_string());
        let restore = should_restore(change_count(), after_copy);
        if restore {
            restore_pasteboard(saved);
        }
        log::debug!("selection capture: pasteboard restored={restore}");
        text
    })
    .await
    .ok()?
    .filter(|text| !text.is_empty());
    log::debug!(
        "selection capture: copy fallback menu item={pressed}, {} ms, length={:?}",
        started.elapsed().as_millis(),
        text.as_ref().map(String::len)
    );
    text
}

async fn read_after_trigger(
    app: &AppHandle,
    generation: u64,
    run: u64,
    baseline: &mut Option<AxSelection<Cf>>,
) -> Option<String> {
    tokio::time::sleep(SETTLE_DELAY).await;
    let read = tauri::async_runtime::spawn_blocking(read_ax_selection).await.ok()?;
    if let Some(read) = read.filter(|read| !read.text.is_empty()) {
        if equals_baseline(baseline.as_ref(), &read) {
            log::debug!("selection capture: AX selection equals the baseline, waiting");
            return None;
        }
        log::debug!("selection capture: AX path, length={}", read.text.len());
        return Some(read.text);
    }
    if !keep_running(app, generation, run).await {
        return None;
    }
    copy_fallback(app, generation, run).await
}

pub async fn capture(app: &AppHandle, generation: u64, run: u64) -> Option<String> {
    if !accessibility_trusted_or_prompt() {
        warn_permission_missing(app).await;
        return None;
    }
    let mut baseline = tauri::async_runtime::spawn_blocking(read_ax_selection).await.ok()?;
    let started = Instant::now();
    let mut detector = TriggerDetector::default();
    loop {
        if !keep_running(app, generation, run).await {
            log::debug!("selection capture stopped after {} ms", started.elapsed().as_millis());
            return None;
        }
        if let Some(sample) = read_sample(started) {
            detector.pill = pill_frame(app);
            if let Some(trigger) = detector.feed(&sample) {
                log::debug!("selection capture: trigger {}", trigger.name());
                if let Some(text) = read_after_trigger(app, generation, run, &mut baseline).await {
                    return Some(text);
                }
            }
        }
        tokio::time::sleep(POLL_INTERVAL).await;
    }
}
