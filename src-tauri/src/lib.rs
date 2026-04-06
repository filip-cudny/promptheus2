use std::collections::HashMap;
use std::sync::RwLock;

use tauri::{Emitter, Manager};
use tokio::sync::Mutex;

mod commands;
mod models;
mod providers;
mod services;
mod traits;

use commands::settings::AppState;
use services::ai::AiService;
use services::hotkeys::ShortcutActionMap;

#[cfg(target_os = "macos")]
fn detect_frontmost_app() -> String {
    let script =
        r#"tell application "System Events" to return name of first application process whose frontmost is true"#;
    std::process::Command::new("osascript")
        .args(["-e", script])
        .output()
        .ok()
        .and_then(|o| {
            if o.status.success() {
                String::from_utf8(o.stdout)
                    .ok()
                    .map(|s| s.trim().to_string())
            } else {
                None
            }
        })
        .unwrap_or_default()
}

#[cfg(target_os = "linux")]
fn detect_frontmost_app() -> String {
    if std::env::var("WAYLAND_DISPLAY").is_ok() {
        return detect_frontmost_app_wayland();
    }
    detect_frontmost_app_x11()
}

#[cfg(target_os = "linux")]
fn detect_frontmost_app_x11() -> String {
    let window_id = std::process::Command::new("xdotool")
        .arg("getactivewindow")
        .output()
        .ok()
        .filter(|o| o.status.success())
        .and_then(|o| String::from_utf8(o.stdout).ok())
        .map(|s| s.trim().to_string())
        .unwrap_or_default();

    if window_id.is_empty() {
        return String::new();
    }

    let pid = std::process::Command::new("xdotool")
        .args(["getwindowpid", &window_id])
        .output()
        .ok()
        .filter(|o| o.status.success())
        .and_then(|o| String::from_utf8(o.stdout).ok())
        .map(|s| s.trim().to_string())
        .unwrap_or_default();

    if !pid.is_empty() {
        if let Ok(comm) = std::fs::read_to_string(format!("/proc/{pid}/comm")) {
            let name = comm.trim().to_string();
            if !name.is_empty() {
                return name;
            }
        }
    }

    std::process::Command::new("xdotool")
        .args(["getwindowname", &window_id])
        .output()
        .ok()
        .filter(|o| o.status.success())
        .and_then(|o| String::from_utf8(o.stdout).ok())
        .map(|s| s.trim().to_string())
        .unwrap_or_default()
}

#[cfg(target_os = "linux")]
fn detect_frontmost_app_wayland() -> String {
    let output = std::process::Command::new("gdbus")
        .args([
            "call",
            "--session",
            "--dest",
            "org.gnome.Shell",
            "--object-path",
            "/org/gnome/Shell",
            "--method",
            "org.gnome.Shell.Eval",
            "global.display.focus_window ? global.display.focus_window.get_wm_class() : ''",
        ])
        .output()
        .ok()
        .filter(|o| o.status.success())
        .and_then(|o| String::from_utf8(o.stdout).ok())
        .unwrap_or_default();

    output
        .split('\'')
        .nth(1)
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .unwrap_or_default()
}

#[cfg(not(any(target_os = "macos", target_os = "linux")))]
fn detect_frontmost_app() -> String {
    String::new()
}
use services::clipboard::ClipboardService;
use services::config::ConfigService;
use services::context::{ContextManagerService, ContextMenuProvider};
use services::database::Database;
use services::sqlite_history::SqliteHistoryService;
use services::image_storage::ImageStorage;
use services::menu_coordinator::MenuCoordinator;
use services::dock::DockManager;
use services::notification::NotificationService;
use services::placeholder::PlaceholderService;
use services::prompt_execution::PromptExecutionService;
use services::skill::SkillService;
use services::speech::SpeechService;
use providers::{LastInteractionMenuProvider, SkillMenuProvider, SpeechMenuProvider};

fn create_app_windows(app: &tauri::App) -> Result<(), Box<dyn std::error::Error>> {
    use tauri::webview::{Color, WebviewWindowBuilder};

    let transparent = cfg!(target_os = "macos");

    let mut cm = WebviewWindowBuilder::new(
        app,
        "context-menu",
        tauri::WebviewUrl::App("context-menu.html".into()),
    )
    .title("")
    .inner_size(320.0, 400.0)
    .resizable(false)
    .decorations(false)
    .transparent(transparent)
    .always_on_top(true)
    .skip_taskbar(true)
    .visible(false);

    if !transparent {
        cm = cm.background_color(Color(0x1e, 0x1e, 0x1e, 0xff));
    }

    cm.build()?;

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
        "text-preview",
        tauri::WebviewUrl::App("text-preview.html".into()),
    )
    .title("Text Preview")
    .inner_size(500.0, 400.0)
    .resizable(true)
    .decorations(true)
    .transparent(false)
    .always_on_top(true)
    .skip_taskbar(true)
    .visible(false)
    .build()?;

    Ok(())
}

fn setup_tray(app: &tauri::App) -> Result<(), Box<dyn std::error::Error>> {
    use tauri::menu::{Menu, MenuItem, PredefinedMenuItem};
    use tauri::tray::TrayIconBuilder;

    let show_menu_i = MenuItem::with_id(app, "show-menu", "Show Menu", true, None::<&str>)?;
    let sep1 = PredefinedMenuItem::separator(app)?;
    let settings_i = MenuItem::with_id(app, "settings", "Settings", true, None::<&str>)?;
    let sep2 = PredefinedMenuItem::separator(app)?;
    let quit_i = MenuItem::with_id(app, "quit", "Quit", true, None::<&str>)?;

    let menu = Menu::with_items(app, &[&show_menu_i, &sep1, &settings_i, &sep2, &quit_i])?;

    let tray_icon_image = tauri::image::Image::from_bytes(include_bytes!("../icons/tray_icon.png"))?;

    TrayIconBuilder::with_id("main-tray")
        .icon(tray_icon_image)
        .menu(&menu)
        .show_menu_on_left_click(true)
        .tooltip("Promptheus")
        .on_menu_event(|app: &tauri::AppHandle, event: tauri::menu::MenuEvent| match event.id().as_ref() {
            "show-menu" => {
                let frontmost = detect_frontmost_app();
                let app = app.clone();
                tauri::async_runtime::spawn(async move {
                    {
                        let state = app.state::<Mutex<AppState>>();
                        state.lock().await.push_active_app(frontmost);
                    }
                    let _ = commands::menu::show_context_menu_window(app).await;
                });
            }
            "settings" => {
                let _ = app.emit("open-settings", ());
            }
            "quit" => {
                app.exit(0);
            }
            _ => {}
        })
        .build(app)?;

    Ok(())
}

async fn execute_context_action(app: &tauri::AppHandle, action: &str) {
    use services::notification::NotificationLevel;

    let state = app.state::<Mutex<AppState>>();
    match action {
        "set_context_value" => {
            let notification_settings = {
                let mut s = state.lock().await;
                let result: Result<(), String> = if s.clipboard.has_image() {
                    s.clipboard
                        .get_image_base64()
                        .map(|(data, mt)| s.context.set_context_image(data, mt))
                        .map_err(|e| e.to_string())
                } else {
                    s.clipboard
                        .get_text()
                        .map(|text| s.context.set_context(text))
                        .map_err(|e| e.to_string())
                };
                if let Err(e) = result {
                    log::error!("set_context_value hotkey failed: {}", e);
                    return;
                }
                s.config.settings().notifications.clone()
            };
            let _ = app.emit("context-changed", ());
            let _ = state.lock().await.notifications.notify(
                "context_set",
                NotificationLevel::Success,
                "Context set",
                None::<String>,
                &notification_settings,
            );
        }
        "append_context_value" => {
            let notification_settings = {
                let mut s = state.lock().await;
                let result: Result<(), String> = if s.clipboard.has_image() {
                    s.clipboard
                        .get_image_base64()
                        .map(|(data, mt)| s.context.append_context_image(data, mt))
                        .map_err(|e| e.to_string())
                } else {
                    s.clipboard
                        .get_text()
                        .map(|text| s.context.append_context(text))
                        .map_err(|e| e.to_string())
                };
                if let Err(e) = result {
                    log::error!("append_context_value hotkey failed: {}", e);
                    return;
                }
                s.config.settings().notifications.clone()
            };
            let _ = app.emit("context-changed", ());
            let _ = state.lock().await.notifications.notify(
                "context_append",
                NotificationLevel::Success,
                "Context appended",
                None::<String>,
                &notification_settings,
            );
        }
        "clear_context" => {
            let notification_settings = {
                let mut s = state.lock().await;
                s.context.clear();
                s.config.settings().notifications.clone()
            };
            let _ = app.emit("context-changed", ());
            let _ = state.lock().await.notifications.notify(
                "context_cleared",
                NotificationLevel::Success,
                "Context cleared",
                None::<String>,
                &notification_settings,
            );
        }
        _ => {}
    }
}

#[cfg(desktop)]
pub fn reload_shortcuts(app: &tauri::AppHandle, settings: &models::settings::Settings) {
    use tauri_plugin_global_shortcut::{GlobalShortcutExt, Shortcut};

    let global_shortcut = app.global_shortcut();

    if let Err(e) = global_shortcut.unregister_all() {
        log::error!("failed to unregister shortcuts: {}", e);
        return;
    }

    let bindings = services::hotkeys::get_active_bindings(settings);
    let mut new_action_map = HashMap::new();

    for (shortcut_str, action) in &bindings {
        match shortcut_str.parse::<Shortcut>() {
            Ok(shortcut) => {
                let canonical = shortcut.into_string();
                new_action_map.insert(canonical.clone(), action.clone());
                if let Err(e) = global_shortcut.register(shortcut) {
                    log::warn!("failed to register shortcut {} ({}): {}", shortcut_str, canonical, e);
                }
            }
            Err(e) => {
                log::warn!("invalid shortcut {}: {}", shortcut_str, e);
            }
        }
    }

    let action_map_state = app.state::<ShortcutActionMap>();
    let mut map = action_map_state.0.write().unwrap();
    *map = new_action_map;

    log::info!("reloaded {} global shortcuts", bindings.len());
}

async fn execute_hotkey_action(app: &tauri::AppHandle, action: &str) {
    match action {
        "set_context_value" | "append_context_value" | "clear_context" => {
            execute_context_action(app, action).await;
        }
        "open_context_menu" => {
            let frontmost = detect_frontmost_app();
            {
                let state = app.state::<Mutex<AppState>>();
                state.lock().await.push_active_app(frontmost);
            }
            if let Err(e) = commands::menu::show_context_menu_window(app.clone()).await {
                log::error!("open_context_menu failed: {e}");
            }
        }
        "speech_to_text_toggle" => {
            let state = app.state::<Mutex<AppState>>();
            if let Err(e) = commands::speech::toggle_speech_recording(
                app.clone(),
                state,
                None,
            ).await {
                log::error!("speech_to_text_toggle failed: {e}");
            }
        }
        "execute_active_prompt" => {
            log::warn!("hotkey action '{}' is not yet implemented", action);
        }
        _ => {
            log::warn!("unknown hotkey action: {}", action);
        }
    }
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(
            tauri_plugin_log::Builder::new()
                .targets([
                    tauri_plugin_log::Target::new(tauri_plugin_log::TargetKind::Stdout),
                    tauri_plugin_log::Target::new(tauri_plugin_log::TargetKind::LogDir {
                        file_name: None,
                    }),
                    tauri_plugin_log::Target::new(tauri_plugin_log::TargetKind::Webview),
                ])
                .timezone_strategy(tauri_plugin_log::TimezoneStrategy::UseLocal)
                .level(log::LevelFilter::Info)
                .level_for("app_lib", log::LevelFilter::Debug)
                .build(),
        )
        .plugin(tauri_plugin_clipboard_manager::init())
        .plugin(tauri_plugin_opener::init())
        .plugin(
            tauri::plugin::Builder::<tauri::Wry, ()>::new("platform")
                .js_init_script(format!(
                    "document.documentElement.dataset.platform = '{}'",
                    std::env::consts::OS
                ))
                .build(),
        )
        .setup(|app| {
            #[cfg(target_os = "macos")]
            app.set_activation_policy(tauri::ActivationPolicy::Accessory);

            let config_dir = app.path().app_config_dir()?;
            let resource_dir =
                app.path()
                    .resolve("", tauri::path::BaseDirectory::Resource)?;
            services::config::load_env(&config_dir);
            let mut config_service =
                ConfigService::load(&config_dir, Some(&resource_dir))
                    .map_err(|e| Box::new(e) as Box<dyn std::error::Error>)?;
            log::info!("config loaded from {}", config_dir.display());

            if config_service.settings().show_tray_icon {
                setup_tray(app)?;
                log::info!("system tray initialized");
            }

            create_app_windows(app)?;

            let bindings = services::hotkeys::get_active_bindings(config_service.settings());

            #[cfg(desktop)]
            {
                use tauri_plugin_global_shortcut::{Shortcut, ShortcutState};

                let mut action_map = HashMap::new();
                let mut builder = tauri_plugin_global_shortcut::Builder::new();
                for (shortcut_str, action) in &bindings {
                    match shortcut_str.parse::<Shortcut>() {
                        Ok(shortcut) => {
                            let canonical = shortcut.into_string();
                            action_map.insert(canonical, action.clone());
                            builder = builder.with_shortcut(shortcut_str.as_str())?;
                        }
                        Err(e) => {
                            log::warn!("invalid shortcut {}: {}", shortcut_str, e);
                        }
                    }
                }
                app.manage(ShortcutActionMap(RwLock::new(action_map)));

                app.handle().plugin(
                    builder
                        .with_handler(|app, shortcut, event| {
                            if event.state == ShortcutState::Pressed {
                                let shortcut_str = shortcut.into_string();
                                let action_map = app.state::<ShortcutActionMap>();
                                let map = action_map.0.read().unwrap();
                                if let Some(action) = map.get(&shortcut_str) {
                                    let action = action.clone();
                                    drop(map);
                                    log::info!("hotkey action: {} -> {}", shortcut_str, action);
                                    let app = app.clone();
                                    tauri::async_runtime::spawn(async move {
                                        execute_hotkey_action(&app, &action).await;
                                    });
                                } else {
                                    log::warn!("hotkey pressed but no action found: {}", shortcut_str);
                                }
                            }
                        })
                        .build(),
                )?;

                for (shortcut_str, action) in &bindings {
                    log::info!("registered shortcut: {} -> {}", shortcut_str, action);
                }
                log::info!("{} global shortcuts registered", bindings.len());
            }

            app.manage(DockManager::new());

            let skills_dir = config_dir.join("skills");
            let mut skill_service = SkillService::load(
                &skills_dir,
                Some(&resource_dir),
                &config_service.settings().skills_order,
            )
            .map_err(|e| Box::new(e) as Box<dyn std::error::Error>)?;

            log::info!(
                "loaded {} skills from {}",
                skill_service.list_skills().len(),
                skills_dir.display()
            );

            let clipboard_service = ClipboardService::new()
                .map_err(|e| Box::new(e) as Box<dyn std::error::Error>)?;
            let notification_service = NotificationService::new(app.handle().clone());
            let mut menu_coordinator = MenuCoordinator::new();
            menu_coordinator.add_provider(Box::new(ContextMenuProvider::new()));
            menu_coordinator.add_provider(Box::new(LastInteractionMenuProvider::new()));
            menu_coordinator.add_provider(Box::new(SpeechMenuProvider::new()));

            let skill_summaries: Vec<_> = skill_service
                .list_skills()
                .iter()
                .map(|s| crate::models::skill::SkillSummary {
                    name: s.name.clone(),
                    display_name: s.display_name.clone(),
                    description: s.description.clone(),
                })
                .collect();
            menu_coordinator.add_provider(Box::new(SkillMenuProvider::new(skill_summaries)));

            let ui_state_service = services::ui_state::UiStateService::load(&config_dir)
                .map_err(|e| Box::new(e) as Box<dyn std::error::Error>)?;

            let ai_service = AiService::new(&config_service.settings().models);
            let context_service = ContextManagerService::new();
            let placeholder_service = PlaceholderService::new();
            let app_data_dir = app.path().app_data_dir()?;
            let database = Database::open(&app_data_dir)
                .map_err(|e| Box::new(e) as Box<dyn std::error::Error>)?;
            let history_service = SqliteHistoryService::new(database, 1000);
            let image_storage = ImageStorage::new(&app_data_dir);
            image_storage
                .initialize()
                .map_err(|e| Box::new(e) as Box<dyn std::error::Error>)?;
            log::info!("image storage initialized at {}", app_data_dir.display());

            app.manage(Mutex::new(AppState {
                config: config_service,
                clipboard: clipboard_service,
                notifications: notification_service,
                menu_coordinator,
                context: context_service,
                placeholder: placeholder_service,
                ai: ai_service,
                history: history_service,
                image_storage,
                prompt_execution: PromptExecutionService::new(),
                skill_service,
                speech: SpeechService::new(),
                ui_state: ui_state_service,
                conversation_context: crate::services::conversation_context::ConversationContextCache::new(),
                tool_confirmation: crate::services::tool_confirmation::ToolConfirmationService::new(),
                recent_apps: std::collections::VecDeque::new(),
            }));
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::ai::complete,
            commands::ai::complete_stream,
            commands::clipboard::get_clipboard_text,
            commands::clipboard::set_clipboard_text,
            commands::clipboard::clipboard_is_empty,
            commands::clipboard::clipboard_has_image,
            commands::clipboard::get_clipboard_image,
            commands::settings::get_settings,
            commands::settings::update_setting,
            commands::settings::add_model,
            commands::settings::update_model,
            commands::settings::update_model_reasoning_effort,
            commands::settings::delete_model,
            commands::settings::update_notifications,
            commands::settings::update_speech_model,
            commands::settings::update_keymaps,
            commands::settings::update_menu_section_order,
            commands::settings::reload_settings,
            commands::context::get_context_items,
            commands::context::get_context_text,
            commands::context::has_context,
            commands::context::has_context_images,
            commands::context::set_context,
            commands::context::append_context,
            commands::context::clear_context,
            commands::context::remove_context_item,
            commands::context::set_context_image,
            commands::context::append_context_image,
            commands::context::set_context_from_clipboard,
            commands::context::append_context_from_clipboard,
            commands::menu::get_context_menu_items,
            commands::menu::execute_menu_item,
            commands::menu::refresh_menu_providers,
            commands::menu::show_context_menu_window,
            commands::menu::focus_context_menu,
            commands::history::get_history,
            commands::history::get_conversations,
            commands::history::get_history_entry,
            commands::history::add_history_entry,
            commands::history::add_conversation_entry,
            commands::history::update_conversation_entry,
            commands::history::get_last_interaction,
            commands::history::clear_history,
            commands::history::copy_history_content,
            commands::history::update_history_entry_title,
            commands::prompt_execution::execute_skill,
            commands::prompt_execution::resolve_environment_section,
            commands::prompt_execution::release_conversation_context,
            commands::prompt_execution::seed_conversation_context,
            commands::prompt_execution::generate_conversation_title,
            commands::prompt_execution::resolve_skill_input,
            commands::prompt_execution::execute_conversation_from_tree,
            commands::prompt_execution::respond_to_tool_call,
            commands::prompt_execution::retry_tool_call,
            commands::conversation_dialog::open_conversation_dialog,
            commands::conversation_dialog::get_dialog_init_params,
            commands::skills::list_skills,
            commands::skills::get_skill,
            commands::skills::get_skill_body,
            commands::skills::reload_skills,
            commands::context_editor::open_context_editor,
            commands::history_dialog::open_history_dialog,
            commands::image_preview::open_image_preview,
            commands::image_preview::get_pending_image,
            commands::image_preview::get_image_preview_work_area,
            commands::text_preview::open_text_preview,
            commands::text_preview::get_pending_text,
            commands::text_preview::save_text_preview_geometry,
            commands::dock::hide_dialog_window,
            commands::notification::update_notification_window,
            commands::notification::drain_pending_notifications,
            commands::speech::toggle_speech_recording,
            commands::speech::get_recording_state,
            commands::tokenizer::count_tokens,
            commands::tokenizer::get_skill_token_counts,
            commands::tokenizer::count_conversation_tokens,
            commands::ui_state::get_ui_state,
            commands::ui_state::set_ui_state,
        ])
        .on_window_event(|window, event| {
            if let tauri::WindowEvent::Focused(false) = event {
                let app = window.app_handle().clone();
                tauri::async_runtime::spawn(async move {
                    tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
                    let detected = detect_frontmost_app();
                    let state = app.state::<Mutex<crate::commands::settings::AppState>>();
                    state.lock().await.push_active_app(detected);
                });
            }
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
