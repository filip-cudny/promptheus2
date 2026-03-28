use std::fs;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use base64::{engine::general_purpose::STANDARD, Engine};

#[derive(Debug, thiserror::Error)]
pub enum ImageStorageError {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    #[error("Base64 decode error: {0}")]
    Base64Decode(#[from] base64::DecodeError),
}

pub struct ImageStorage {
    temp_dir: PathBuf,
}

impl ImageStorage {
    pub fn new(app_data_dir: &Path) -> Self {
        Self {
            temp_dir: app_data_dir.join("temp_images"),
        }
    }

    pub fn initialize(&self) -> Result<(), ImageStorageError> {
        if self.temp_dir.exists() {
            fs::remove_dir_all(&self.temp_dir)?;
        }
        fs::create_dir_all(&self.temp_dir)?;
        Ok(())
    }

    pub fn save_image(
        &self,
        base64_data: &str,
        media_type: &str,
    ) -> Result<String, ImageStorageError> {
        fs::create_dir_all(&self.temp_dir)?;

        let extension = Self::extension_for_media_type(media_type);
        let content_hash = Self::hash_string(base64_data);
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_millis();
        let filename = format!("img_{timestamp}_{content_hash}.{extension}");

        let filepath = self.temp_dir.join(&filename);
        let image_bytes = STANDARD.decode(base64_data)?;
        fs::write(&filepath, image_bytes)?;

        Ok(filepath.to_string_lossy().to_string())
    }

    pub fn load_image(&self, filepath: &str) -> Result<(String, String), ImageStorageError> {
        let path = Path::new(filepath);
        let image_bytes = fs::read(path)?;
        let base64_data = STANDARD.encode(&image_bytes);

        let ext = path
            .extension()
            .and_then(|e| e.to_str())
            .unwrap_or("png");
        let media_type = Self::media_type_for_extension(ext).to_string();

        Ok((base64_data, media_type))
    }

    pub fn cleanup(&self) -> Result<(), ImageStorageError> {
        if self.temp_dir.exists() {
            fs::remove_dir_all(&self.temp_dir)?;
            fs::create_dir_all(&self.temp_dir)?;
        }
        Ok(())
    }

    fn extension_for_media_type(media_type: &str) -> &'static str {
        match media_type.to_lowercase().as_str() {
            "image/png" => "png",
            "image/jpeg" | "image/jpg" => "jpg",
            "image/gif" => "gif",
            "image/webp" => "webp",
            "image/bmp" => "bmp",
            _ => "png",
        }
    }

    fn media_type_for_extension(ext: &str) -> &'static str {
        match ext.to_lowercase().as_str() {
            "png" => "image/png",
            "jpg" | "jpeg" => "image/jpeg",
            "gif" => "image/gif",
            "webp" => "image/webp",
            "bmp" => "image/bmp",
            _ => "image/png",
        }
    }

    fn hash_string(input: &str) -> String {
        let mut hash: u64 = 0;
        for byte in input.bytes() {
            hash = hash.wrapping_mul(31).wrapping_add(byte as u64);
        }
        format!("{hash:012x}")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn setup() -> (tempfile::TempDir, ImageStorage) {
        let tmp = tempfile::tempdir().unwrap();
        let storage = ImageStorage::new(tmp.path());
        storage.initialize().unwrap();
        (tmp, storage)
    }

    fn sample_png_base64() -> String {
        let pixel: [u8; 4] = [255, 0, 0, 255];
        STANDARD.encode(&pixel)
    }

    #[test]
    fn save_and_load_round_trip() {
        let (_tmp, storage) = setup();
        let original_b64 = sample_png_base64();

        let path = storage.save_image(&original_b64, "image/png").unwrap();
        assert!(Path::new(&path).exists());
        assert!(path.ends_with(".png"));

        let (loaded_b64, media_type) = storage.load_image(&path).unwrap();
        assert_eq!(loaded_b64, original_b64);
        assert_eq!(media_type, "image/png");
    }

    #[test]
    fn initialize_clears_existing_files() {
        let (_tmp, storage) = setup();

        storage.save_image(&sample_png_base64(), "image/png").unwrap();
        assert!(fs::read_dir(&storage.temp_dir).unwrap().count() > 0);

        storage.initialize().unwrap();
        assert_eq!(fs::read_dir(&storage.temp_dir).unwrap().count(), 0);
    }

    #[test]
    fn cleanup_removes_files() {
        let (_tmp, storage) = setup();

        storage.save_image(&sample_png_base64(), "image/png").unwrap();
        storage.save_image(&sample_png_base64(), "image/jpeg").unwrap();
        assert!(fs::read_dir(&storage.temp_dir).unwrap().count() >= 1);

        storage.cleanup().unwrap();
        assert_eq!(fs::read_dir(&storage.temp_dir).unwrap().count(), 0);
    }

    #[test]
    fn extension_for_all_media_types() {
        assert_eq!(ImageStorage::extension_for_media_type("image/png"), "png");
        assert_eq!(ImageStorage::extension_for_media_type("image/jpeg"), "jpg");
        assert_eq!(ImageStorage::extension_for_media_type("image/jpg"), "jpg");
        assert_eq!(ImageStorage::extension_for_media_type("image/gif"), "gif");
        assert_eq!(ImageStorage::extension_for_media_type("image/webp"), "webp");
        assert_eq!(ImageStorage::extension_for_media_type("image/bmp"), "bmp");
        assert_eq!(ImageStorage::extension_for_media_type("unknown/type"), "png");
    }

    #[test]
    fn media_type_for_all_extensions() {
        assert_eq!(ImageStorage::media_type_for_extension("png"), "image/png");
        assert_eq!(ImageStorage::media_type_for_extension("jpg"), "image/jpeg");
        assert_eq!(ImageStorage::media_type_for_extension("jpeg"), "image/jpeg");
        assert_eq!(ImageStorage::media_type_for_extension("gif"), "image/gif");
        assert_eq!(ImageStorage::media_type_for_extension("webp"), "image/webp");
        assert_eq!(ImageStorage::media_type_for_extension("bmp"), "image/bmp");
        assert_eq!(ImageStorage::media_type_for_extension("xyz"), "image/png");
    }

    #[test]
    fn filename_format() {
        let (_tmp, storage) = setup();
        let path = storage.save_image(&sample_png_base64(), "image/jpeg").unwrap();
        let filename = Path::new(&path).file_name().unwrap().to_str().unwrap();

        assert!(filename.starts_with("img_"));
        assert!(filename.ends_with(".jpg"));
        let parts: Vec<&str> = filename.strip_prefix("img_").unwrap()
            .strip_suffix(".jpg").unwrap()
            .splitn(2, '_')
            .collect();
        assert_eq!(parts.len(), 2);
        assert!(parts[0].parse::<u128>().is_ok());
        assert_eq!(parts[1].len(), 12);
    }

    #[test]
    fn load_nonexistent_file_returns_error() {
        let (_tmp, storage) = setup();
        let result = storage.load_image("/nonexistent/path.png");
        assert!(result.is_err());
    }
}
