#![cfg(target_os = "macos")]

use std::ffi::CStr;
use std::sync::Once;

use objc2::ffi::object_setClass;
use objc2::runtime::{AnyClass, AnyObject, Bool, ClassBuilder, Sel};
use objc2::{class, msg_send, sel};
use objc2_app_kit::{NSPanel, NSWindow, NSWindowStyleMask};

// Custom NSPanel subclass: canBecomeKeyWindow=YES, canBecomeMainWindow=NO.
// OR'ing NonactivatingPanel into the style mask after object_setClass is not
// enough — Cocoa caches some behavior at init-time and the panel either fails
// to become key (keystrokes leak to the frontmost app) or pulls the whole app
// forward. Overriding the class methods directly nails down both behaviors.
fn promptheus_panel_class() -> &'static AnyClass {
    static REGISTER: Once = Once::new();
    static mut CLASS: Option<&'static AnyClass> = None;

    REGISTER.call_once(|| {
        let superclass = class!(NSPanel);
        let name = CStr::from_bytes_with_nul(b"PromptheusContextMenuPanel\0").unwrap();
        let mut builder = ClassBuilder::new(name, superclass)
            .expect("PromptheusContextMenuPanel class registration failed");

        extern "C-unwind" fn can_become_key_window(_: &AnyObject, _: Sel) -> Bool {
            Bool::YES
        }

        extern "C-unwind" fn can_become_main_window(_: &AnyObject, _: Sel) -> Bool {
            Bool::NO
        }

        unsafe {
            let key_fn: extern "C-unwind" fn(_, _) -> _ = can_become_key_window;
            builder.add_method(sel!(canBecomeKeyWindow), key_fn);
            let main_fn: extern "C-unwind" fn(_, _) -> _ = can_become_main_window;
            builder.add_method(sel!(canBecomeMainWindow), main_fn);
            CLASS = Some(builder.register());
        }
    });

    unsafe { CLASS.expect("PromptheusContextMenuPanel class missing") }
}

// Convert a webview window's underlying NSWindow into a non-activating NSPanel
// so that showing the panel does not raise the app's other windows.
pub fn make_nonactivating_panel(window: &tauri::WebviewWindow) -> Result<(), String> {
    let ptr = window.ns_window().map_err(|e| e.to_string())?;
    if ptr.is_null() {
        return Err("ns_window pointer is null".into());
    }

    unsafe {
        object_setClass(ptr as *mut AnyObject, promptheus_panel_class());

        let panel: &NSPanel = &*(ptr as *const NSPanel);
        let mask = panel.styleMask();
        panel.setStyleMask(mask | NSWindowStyleMask::NonactivatingPanel);
        panel.setBecomesKeyOnlyIfNeeded(false);
        panel.setFloatingPanel(true);
    }

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
