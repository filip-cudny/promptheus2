#[tauri::command]
pub async fn write_text_file(path: String, content: String) -> crate::Result<()> {
    tokio::fs::write(&path, content).await?;
    Ok(())
}
