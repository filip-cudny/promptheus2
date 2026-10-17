use crate::services::notification::NotificationPayload;
use crate::services::notification_window;

#[tauri::command]
pub fn drain_pending_notifications() -> Vec<NotificationPayload> {
    notification_window::drain_pending_notifications()
}

#[tauri::command]
pub async fn update_notification_window(
    app: tauri::AppHandle,
    count: u32,
    height: u32,
) -> crate::Result<()> {
    notification_window::update_notification_window(app, count, height).await
}
