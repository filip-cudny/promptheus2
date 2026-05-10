pub mod ai;
pub mod ai_webview;
pub mod clipboard;
pub mod context;
pub mod context_editor;
pub mod dock;
pub mod execution_control;
pub mod execution_generation;
pub mod execution_stream;
pub mod history;
pub mod history_dialog;
pub mod image_preview;
pub mod mcp;
pub mod menu;
pub mod notification;
pub mod prompts;
pub mod provider_menu;
pub mod conversation_dialog;
pub mod settings;
pub mod settings_dialog;
pub mod skills;
pub mod speech;
pub mod text_preview;
pub mod tokenizer;
pub mod ui_state;

#[macro_export]
macro_rules! handlers {
    () => {
        tauri::generate_handler![
            // === ai ===
            $crate::commands::ai::complete,
            $crate::commands::ai::complete_stream,
            $crate::commands::ai::get_model_capabilities,
            // === ai_webview ===
            $crate::commands::ai_webview::open_ai_webview,
            $crate::commands::ai_webview::open_ai_webview_new_window,
            $crate::commands::ai_webview::swap_ai_webview,
            $crate::commands::ai_webview::swap_to_conversation_dialog,
            $crate::commands::ai_webview::navigate_ai_webview,
            $crate::commands::ai_webview::close_ai_webview,
            $crate::commands::ai_webview::get_webview_providers,
            $crate::commands::ai_webview::get_webview_provider,
            $crate::commands::ai_webview::get_active_provider,
            $crate::commands::ai_webview::take_pending_provider,
            $crate::commands::ai_webview::new_chat_in_host,
            $crate::commands::ai_webview::reload_active_in_host,
            $crate::commands::ai_webview::open_palette,
            $crate::commands::ai_webview::close_palette,
            // === clipboard ===
            $crate::commands::clipboard::get_clipboard_text,
            $crate::commands::clipboard::set_clipboard_text,
            $crate::commands::clipboard::clipboard_is_empty,
            $crate::commands::clipboard::clipboard_has_image,
            $crate::commands::clipboard::get_clipboard_image,
            // === context ===
            $crate::commands::context::get_context_items,
            $crate::commands::context::get_context_text,
            $crate::commands::context::has_context,
            $crate::commands::context::has_context_images,
            $crate::commands::context::set_context,
            $crate::commands::context::append_context,
            $crate::commands::context::clear_context,
            $crate::commands::context::remove_context_item,
            $crate::commands::context::set_context_image,
            $crate::commands::context::append_context_image,
            $crate::commands::context::set_context_from_clipboard,
            $crate::commands::context::append_context_from_clipboard,
            // === context_editor ===
            $crate::commands::context_editor::open_context_editor,
            // === conversation_dialog ===
            $crate::commands::conversation_dialog::open_conversation_dialog,
            $crate::commands::conversation_dialog::open_conversation_dialog_new_window,
            $crate::commands::conversation_dialog::focus_or_open_chat,
            $crate::commands::conversation_dialog::get_dialog_init_params,
            // === dock ===
            $crate::commands::dock::hide_dialog_window,
            // === execution_stream ===
            $crate::commands::execution_stream::execute_skill,
            $crate::commands::execution_stream::execute_conversation_from_tree,
            $crate::commands::execution_stream::resolve_environment_section,
            $crate::commands::execution_stream::release_conversation_context,
            $crate::commands::execution_stream::seed_conversation_context,
            $crate::commands::execution_stream::resolve_skill_input,
            // === execution_control ===
            $crate::commands::execution_control::reconnect_to_execution,
            $crate::commands::execution_control::cancel_skill_execution,
            $crate::commands::execution_control::cancel_live_execution,
            $crate::commands::execution_control::get_executing_skill_id,
            $crate::commands::execution_control::respond_to_tool_call,
            $crate::commands::execution_control::retry_tool_call,
            // === execution_generation ===
            $crate::commands::execution_generation::generate_conversation_title,
            // === history ===
            $crate::commands::history::get_history,
            $crate::commands::history::get_conversations,
            $crate::commands::history::get_history_entry,
            $crate::commands::history::add_history_entry,
            $crate::commands::history::add_conversation_entry,
            $crate::commands::history::update_conversation_entry,
            $crate::commands::history::get_last_interaction,
            $crate::commands::history::delete_history_entry,
            $crate::commands::history::clear_history,
            $crate::commands::history::copy_history_content,
            $crate::commands::history::update_history_entry_title,
            $crate::commands::history::search_history,
            $crate::commands::history::list_history_skills,
            // === history_dialog ===
            $crate::commands::history_dialog::open_history_dialog,
            // === image_preview ===
            $crate::commands::image_preview::open_image_preview,
            $crate::commands::image_preview::get_pending_image,
            $crate::commands::image_preview::get_image_preview_work_area,
            // === mcp ===
            $crate::commands::mcp::list_mcp_tools,
            // === menu ===
            $crate::commands::menu::get_context_menu_items,
            $crate::commands::menu::execute_menu_item,
            $crate::commands::menu::refresh_menu_providers,
            $crate::commands::menu::show_context_menu_window,
            $crate::commands::menu::show_context_menu_panel,
            $crate::commands::menu::hide_context_menu_panel,
            $crate::commands::menu::focus_context_menu,
            // === notification ===
            $crate::commands::notification::update_notification_window,
            $crate::commands::notification::drain_pending_notifications,
            // === prompts ===
            $crate::commands::prompts::list_prompts,
            $crate::commands::prompts::get_prompt,
            $crate::commands::prompts::save_prompt,
            $crate::commands::prompts::get_environment_placeholders,
            // === provider_menu ===
            $crate::commands::provider_menu::show_provider_menu,
            $crate::commands::provider_menu::hide_provider_menu,
            $crate::commands::provider_menu::size_provider_menu,
            $crate::commands::provider_menu::provider_menu_select,
            // === settings ===
            $crate::commands::settings::get_settings,
            $crate::commands::settings::update_setting,
            $crate::commands::settings::update_surface_model,
            $crate::commands::settings::update_surface_parameter,
            $crate::commands::settings::update_surface_enabled_tools,
            $crate::commands::settings::update_speech_to_text_config,
            $crate::commands::settings::add_model,
            $crate::commands::settings::update_model,
            $crate::commands::settings::delete_model,
            $crate::commands::settings::update_notifications,
            $crate::commands::settings::update_keymaps,
            $crate::commands::settings::update_menu_section_order,
            $crate::commands::settings::reload_settings,
            // === settings_dialog ===
            $crate::commands::settings_dialog::open_settings_window,
            $crate::commands::settings_dialog::check_env_var,
            // === skills ===
            $crate::commands::skills::list_skills,
            $crate::commands::skills::list_skills_full,
            $crate::commands::skills::get_skill,
            $crate::commands::skills::get_skill_body,
            $crate::commands::skills::reload_skills,
            $crate::commands::skills::create_skill,
            $crate::commands::skills::update_skill,
            $crate::commands::skills::delete_skill,
            $crate::commands::skills::duplicate_skill,
            $crate::commands::skills::reorder_skills,
            $crate::commands::skills::validate_skill_slug,
            $crate::commands::skills::import_skill_file,
            $crate::commands::skills::export_skill,
            $crate::commands::skills::preview_skill_message,
            // === speech ===
            $crate::commands::speech::toggle_speech_recording,
            $crate::commands::speech::get_recording_state,
            $crate::commands::speech::get_stt_keyterms,
            $crate::commands::speech::save_stt_keyterms,
            // === text_preview ===
            $crate::commands::text_preview::open_text_preview,
            $crate::commands::text_preview::get_pending_text,
            $crate::commands::text_preview::save_text_preview_geometry,
            // === tokenizer ===
            $crate::commands::tokenizer::count_tokens,
            $crate::commands::tokenizer::get_skill_token_counts,
            $crate::commands::tokenizer::count_conversation_tokens,
            // === ui_state ===
            $crate::commands::ui_state::get_ui_state,
            $crate::commands::ui_state::set_ui_state,
        ]
    };
}
