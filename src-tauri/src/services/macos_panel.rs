#![cfg(target_os = "macos")]

use objc2::msg_send;
use objc2::runtime::AnyObject;
use objc2_app_kit::{NSPanel, NSWindow, NSWindowStyleMask};

// Convert a webview window's underlying NSWindow into a non-activating NSPanel.
//
// Why: tauri/tao's `set_focus` calls `[NSApp activateIgnoringOtherApps:YES]`,
// which raises every visible window of the host app — so opening the context
// menu while the conversation dialog is open would yank that dialog over
// whatever app the user was working in. A non-activating panel can become the
// key window without activating its app, so it floats over the foreground app
// without disturbing sibling windows.
pub fn make_nonactivating_panel(window: &tauri::WebviewWindow) -> Result<(), String> {
    let ptr = window.ns_window().map_err(|e| e.to_string())?;
    if ptr.is_null() {
        return Err("ns_window pointer is null".into());
    }

    unsafe {
        let obj = ptr as *mut AnyObject;
        let panel_class = objc2::class!(NSPanel);
        let _: () = msg_send![obj, setClass: panel_class];

        let panel: &NSPanel = &*(ptr as *const NSPanel);
        let mask = panel.styleMask();
        panel.setStyleMask(mask | NSWindowStyleMask::NonactivatingPanel);
        panel.setBecomesKeyOnlyIfNeeded(false);
        panel.setFloatingPanel(true);
    }

    Ok(())
}

// Bring a window forward and make it the key window without activating its
// app. Pair with `make_nonactivating_panel` so the panel can receive keyboard
// events while another app stays frontmost.
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
