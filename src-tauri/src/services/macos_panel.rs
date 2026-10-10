#![cfg(target_os = "macos")]

use std::ffi::CStr;
use std::sync::OnceLock;

use objc2::ffi::object_setClass;
use objc2::runtime::{AnyClass, AnyObject, Bool, ClassBuilder, MethodImplementation, Sel};
use objc2::{class, msg_send, sel};
use objc2_app_kit::{NSPanel, NSWindow, NSWindowCollectionBehavior, NSWindowStyleMask};

extern "C-unwind" fn returns_yes(_: &AnyObject, _: Sel) -> Bool {
    Bool::YES
}

extern "C-unwind" fn returns_no(_: &AnyObject, _: Sel) -> Bool {
    Bool::NO
}

// Custom NSPanel subclasses: canBecomeMainWindow=NO, canBecomeKeyWindow=YES for
// the context menu and NO for the recording widget.
// OR'ing NonactivatingPanel into the style mask after object_setClass is not
// enough — Cocoa caches some behavior at init-time and the panel either fails
// to become key (keystrokes leak to the frontmost app) or pulls the whole app
// forward. Overriding the class methods directly nails down both behaviors.
fn register_panel_class(
    name: &'static CStr,
    can_become_key_window: impl MethodImplementation<Callee = AnyObject, Arguments = (), Return = Bool>,
) -> &'static AnyClass {
    let mut builder = ClassBuilder::new(name, class!(NSPanel))
        .unwrap_or_else(|| panic!("{name:?} class registration failed"));

    unsafe {
        builder.add_method(sel!(canBecomeKeyWindow), can_become_key_window);
        let main_fn: extern "C-unwind" fn(_, _) -> _ = returns_no;
        builder.add_method(sel!(canBecomeMainWindow), main_fn);
    }

    builder.register()
}

fn promptheus_panel_class() -> &'static AnyClass {
    static CLASS: OnceLock<&'static AnyClass> = OnceLock::new();
    CLASS.get_or_init(|| {
        let key_fn: extern "C-unwind" fn(_, _) -> _ = returns_yes;
        register_panel_class(c"PromptheusContextMenuPanel", key_fn)
    })
}

fn promptheus_widget_panel_class() -> &'static AnyClass {
    static CLASS: OnceLock<&'static AnyClass> = OnceLock::new();
    CLASS.get_or_init(|| {
        let key_fn: extern "C-unwind" fn(_, _) -> _ = returns_no;
        register_panel_class(c"PromptheusWidgetPanel", key_fn)
    })
}

fn reclass_as_nonactivating_panel(
    window: &tauri::WebviewWindow,
    class: &'static AnyClass,
) -> Result<&'static NSPanel, String> {
    let ptr = window.ns_window().map_err(|e| e.to_string())?;
    if ptr.is_null() {
        return Err("ns_window pointer is null".into());
    }

    unsafe {
        object_setClass(ptr as *mut AnyObject, class);

        let panel: &NSPanel = &*(ptr as *const NSPanel);
        let mask = panel.styleMask();
        panel.setStyleMask(mask | NSWindowStyleMask::NonactivatingPanel);
        Ok(panel)
    }
}

// Convert a webview window's underlying NSWindow into a non-activating NSPanel
// so that showing the panel does not raise the app's other windows.
pub fn make_nonactivating_panel(window: &tauri::WebviewWindow) -> Result<(), String> {
    let panel = reclass_as_nonactivating_panel(window, promptheus_panel_class())?;
    panel.setBecomesKeyOnlyIfNeeded(false);
    panel.setFloatingPanel(true);
    Ok(())
}

// Convert the recording widget into a non-activating NSPanel that never becomes
// key or main and stays visible when the app deactivates, so a click on it
// leaves the frontmost app active.
pub fn make_nonactivating_widget_panel(window: &tauri::WebviewWindow) -> Result<(), String> {
    let panel = reclass_as_nonactivating_panel(window, promptheus_widget_panel_class())?;
    panel.setHidesOnDeactivate(false);
    panel.setBecomesKeyOnlyIfNeeded(false);
    panel.setFloatingPanel(true);
    Ok(())
}

// Order the panel front and grab system-wide focus without raising siblings.
//
// `makeKeyAndOrderFront:` alone is not enough: a NonactivatingPanel becomes the
// app's key window but does not make the app frontmost, so keystrokes still go
// to whatever app was frontmost. We follow with NSRunningApplication's
// cooperative-activation API (no `.activateAllWindows`), which makes the
// process frontmost and routes the keystrokes to our panel without raising
// our other windows.
pub fn show_panel_without_activating(window: &tauri::WebviewWindow) -> Result<(), String> {
    let ptr = window.ns_window().map_err(|e| e.to_string())?;
    if ptr.is_null() {
        return Err("ns_window pointer is null".into());
    }

    unsafe {
        let ns_window: &NSWindow = &*(ptr as *const NSWindow);
        ns_window.makeKeyAndOrderFront(None);

        let running: *mut AnyObject =
            msg_send![class!(NSRunningApplication), currentApplication];
        if !running.is_null() {
            let _: bool = msg_send![running, activateWithOptions: 0usize];
        }
    }

    Ok(())
}

pub fn hide_panel(window: &tauri::WebviewWindow) -> Result<(), String> {
    let ptr = window.ns_window().map_err(|e| e.to_string())?;
    if ptr.is_null() {
        return Err("ns_window pointer is null".into());
    }

    unsafe {
        let ns_window: &NSWindow = &*(ptr as *const NSWindow);
        ns_window.orderOut(None);
    }

    Ok(())
}

// Force the green traffic-light button into native fullscreen-arrows mode.
// Tauri's `TitleBarStyle::Overlay` + `hidden_title(true)` leaves the window
// without `FullScreenPrimary` collection behavior, so AppKit falls back to
// "+" (zoom) by default. Setting it explicitly restores the native default:
// arrows on hover, "+" on Option+hover.
pub fn enable_fullscreen_primary(window: &tauri::Window) -> Result<(), String> {
    let ptr = window.ns_window().map_err(|e| e.to_string())?;
    if ptr.is_null() {
        return Err("ns_window pointer is null".into());
    }

    unsafe {
        let ns_window: &NSWindow = &*(ptr as *const NSWindow);
        let mut behavior = ns_window.collectionBehavior();
        behavior.remove(NSWindowCollectionBehavior::FullScreenNone);
        behavior.remove(NSWindowCollectionBehavior::FullScreenAuxiliary);
        behavior.insert(NSWindowCollectionBehavior::FullScreenPrimary);
        ns_window.setCollectionBehavior(behavior);
    }

    Ok(())
}

// Re-focus the (already visible) panel after a geometry change.
pub fn make_key_without_activating(window: &tauri::WebviewWindow) -> Result<(), String> {
    let ptr = window.ns_window().map_err(|e| e.to_string())?;
    if ptr.is_null() {
        return Err("ns_window pointer is null".into());
    }

    unsafe {
        let ns_window: &NSWindow = &*(ptr as *const NSWindow);
        ns_window.makeKeyAndOrderFront(None);
    }

    Ok(())
}
