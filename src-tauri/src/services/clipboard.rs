use base64::Engine;
use image::ImageBuffer;
use std::io::Cursor;

#[derive(Debug, thiserror::Error)]
pub enum ClipboardError {
    #[error("clipboard unavailable: {0}")]
    Unavailable(String),

    #[error("clipboard error: {0}")]
    Access(String),

    #[error("image conversion error: {0}")]
    ImageConversion(String),
}

pub struct ClipboardService;

impl ClipboardService {
    pub fn new() -> Result<Self, ClipboardError> {
        arboard::Clipboard::new()
            .map_err(|e| ClipboardError::Access(e.to_string()))?;
        Ok(Self)
    }

    pub fn get_text(&self) -> Result<String, ClipboardError> {
        let mut clipboard = arboard::Clipboard::new()
            .map_err(|e| ClipboardError::Access(e.to_string()))?;

        let text = clipboard
            .get_text()
            .map_err(|e| ClipboardError::Unavailable(e.to_string()))?;

        let trimmed = text.trim().to_string();
        if trimmed.is_empty() {
            return Err(ClipboardError::Unavailable("clipboard is empty".into()));
        }

        Ok(trimmed)
    }

    pub fn set_text(&self, content: &str) -> Result<(), ClipboardError> {
        let mut clipboard = arboard::Clipboard::new()
            .map_err(|e| ClipboardError::Access(e.to_string()))?;

        clipboard
            .set_text(content)
            .map_err(|e| ClipboardError::Access(e.to_string()))
    }

    pub fn is_empty(&self) -> bool {
        self.get_text().is_err()
    }

    pub fn has_image(&self) -> bool {
        let Ok(mut clipboard) = arboard::Clipboard::new() else {
            return false;
        };
        clipboard.get_image().is_ok()
    }

    pub fn get_image_base64(&self) -> Result<(String, String), ClipboardError> {
        let mut clipboard = arboard::Clipboard::new()
            .map_err(|e| ClipboardError::Access(e.to_string()))?;

        let image_data = clipboard
            .get_image()
            .map_err(|e| ClipboardError::Unavailable(e.to_string()))?;

        let rgba: ImageBuffer<image::Rgba<u8>, Vec<u8>> = ImageBuffer::from_raw(
            image_data.width as u32,
            image_data.height as u32,
            image_data.bytes.into_owned(),
        )
        .ok_or_else(|| {
            ClipboardError::ImageConversion("failed to create image buffer".into())
        })?;

        let mut png_bytes = Cursor::new(Vec::new());
        rgba.write_to(&mut png_bytes, image::ImageFormat::Png)
            .map_err(|e| ClipboardError::ImageConversion(e.to_string()))?;

        let base64_data =
            base64::engine::general_purpose::STANDARD.encode(png_bytes.into_inner());

        Ok((base64_data, "image/png".into()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    #[ignore]
    fn create_clipboard_service() {
        let service = ClipboardService::new();
        assert!(service.is_ok());
    }

    #[test]
    fn error_display() {
        let unavailable = ClipboardError::Unavailable("empty".into());
        assert_eq!(unavailable.to_string(), "clipboard unavailable: empty");

        let access = ClipboardError::Access("denied".into());
        assert_eq!(access.to_string(), "clipboard error: denied");

        let conversion = ClipboardError::ImageConversion("bad format".into());
        assert_eq!(
            conversion.to_string(),
            "image conversion error: bad format"
        );
    }
}
