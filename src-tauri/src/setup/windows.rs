use tauri::webview::WebviewWindowBuilder;

pub fn create(app: &tauri::App) -> std::result::Result<(), Box<dyn std::error::Error>> {
    let context_menu_window = WebviewWindowBuilder::new(
        app,
        "context-menu",
        tauri::WebviewUrl::App("context-menu.html".into()),
    )
    .title("")
    .inner_size(320.0, 400.0)
    .resizable(false)
    .decorations(false)
    .transparent(true)
    .always_on_top(true)
    .skip_taskbar(true)
    .visible(false)
    .build()?;

    #[cfg(target_os = "macos")]
    {
        if let Err(e) = crate::services::macos_panel::make_nonactivating_panel(&context_menu_window)
        {
            log::warn!("failed to convert context-menu to non-activating panel: {e}");
        }
    }
    #[cfg(not(target_os = "macos"))]
    let _ = context_menu_window;

    let notif = WebviewWindowBuilder::new(
        app,
        "notification",
        tauri::WebviewUrl::App("notification.html".into()),
    )
    .title("")
    .inner_size(380.0, 100.0)
    .resizable(false)
    .decorations(false)
    .transparent(true)
    .shadow(false)
    .always_on_top(true)
    .skip_taskbar(true)
    .focusable(false)
    .visible(false);

    notif.build()?;

    WebviewWindowBuilder::new(
        app,
        "image-preview",
        tauri::WebviewUrl::App("image-preview.html".into()),
    )
    .title("")
    .inner_size(800.0, 800.0)
    .resizable(false)
    .decorations(false)
    .transparent(true)
    .shadow(false)
    .always_on_top(true)
    .skip_taskbar(true)
    .visible(false)
    .build()?;

    WebviewWindowBuilder::new(
        app,
        "provider-menu",
        tauri::WebviewUrl::App("provider-menu.html".into()),
    )
    .title("")
    .inner_size(180.0, 120.0)
    .resizable(false)
    .decorations(false)
    .transparent(true)
    .always_on_top(true)
    .skip_taskbar(true)
    .visible(false)
    .build()?;

    WebviewWindowBuilder::new(
        app,
        "palette-backdrop",
        tauri::WebviewUrl::App("palette-backdrop.html".into()),
    )
    .title("")
    .inner_size(800.0, 600.0)
    .resizable(false)
    .decorations(false)
    .transparent(true)
    .always_on_top(true)
    .skip_taskbar(true)
    .focusable(false)
    .shadow(false)
    .visible(false)
    .build()?;

    WebviewWindowBuilder::new(app, "palette", tauri::WebviewUrl::App("palette.html".into()))
        .title("")
        .inner_size(520.0, 400.0)
        .resizable(false)
        .decorations(false)
        .transparent(true)
        .always_on_top(true)
        .skip_taskbar(true)
        .shadow(false)
        .visible(false)
        .build()?;

    #[cfg(target_os = "linux")]
    configure_overlay_windows_linux(app);

    Ok(())
}

#[cfg(target_os = "linux")]
fn configure_overlay_windows_linux(app: &tauri::App) {
    use gtk::gdk::WindowTypeHint;
    use gtk::prelude::GtkWindowExt;
    use tauri::Manager;

    if let Some(backdrop) = app.get_webview_window("palette-backdrop") {
        if let Ok(gtk_win) = backdrop.gtk_window() {
            gtk_win.set_type_hint(WindowTypeHint::Utility);
            gtk_win.set_skip_pager_hint(true);
            gtk_win.set_skip_taskbar_hint(true);
            gtk_win.set_accept_focus(false);
        }
    }

    if let Some(palette) = app.get_webview_window("palette") {
        if let Ok(gtk_win) = palette.gtk_window() {
            gtk_win.set_type_hint(WindowTypeHint::Utility);
            gtk_win.set_skip_pager_hint(true);
            gtk_win.set_skip_taskbar_hint(true);
        }
    }

    if let Some(ctx) = app.get_webview_window("context-menu") {
        if let Ok(gtk_win) = ctx.gtk_window() {
            gtk_win.set_type_hint(WindowTypeHint::Utility);
            gtk_win.set_skip_pager_hint(true);
            gtk_win.set_skip_taskbar_hint(true);
        }
    }
}
