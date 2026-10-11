use std::collections::HashSet;
use std::hash::{DefaultHasher, Hash, Hasher};
use std::io;
use std::path::{Path, PathBuf};

use base64::Engine;
use image::ImageFormat;

pub const DIR_NAME: &str = "pill-previews";
const MAX_WIDTH: u32 = 480;
const MAX_HEIGHT: u32 = 320;

pub fn preview_file_name(data: &str) -> String {
    let mut hasher = DefaultHasher::new();
    data.hash(&mut hasher);
    format!("{:016x}.png", hasher.finish())
}

pub fn write_preview(dir: &Path, data: &str, media_type: &str) -> io::Result<PathBuf> {
    let path = dir.join(preview_file_name(data));
    if path.exists() {
        return Ok(path);
    }
    let bytes = base64::engine::general_purpose::STANDARD
        .decode(data)
        .map_err(io::Error::other)?;
    if media_type == "image/png" {
        let source = image::load_from_memory_with_format(&bytes, ImageFormat::Png).map_err(io::Error::other)?;
        let fitted = if source.width() > MAX_WIDTH || source.height() > MAX_HEIGHT {
            source.thumbnail(MAX_WIDTH, MAX_HEIGHT)
        } else {
            source
        };
        fitted.save_with_format(&path, ImageFormat::Png).map_err(io::Error::other)?;
    } else {
        std::fs::write(&path, bytes)?;
    }
    Ok(path)
}

pub fn sync_previews(dir: &Path, images: &[(String, String)]) -> io::Result<Vec<PathBuf>> {
    std::fs::create_dir_all(dir)?;
    let paths = images
        .iter()
        .map(|(data, media_type)| write_preview(dir, data, media_type))
        .collect::<io::Result<Vec<_>>>()?;
    let keep: HashSet<&PathBuf> = paths.iter().collect();
    for entry in std::fs::read_dir(dir)? {
        let path = entry?.path();
        if path.is_file() && !keep.contains(&path) {
            std::fs::remove_file(path)?;
        }
    }
    Ok(paths)
}

pub fn clear_previews(dir: &Path) {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return;
    };
    for path in entries.flatten().map(|entry| entry.path()) {
        if path.is_file() {
            let _ = std::fs::remove_file(path);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn png_base64(width: u32, height: u32, shade: u8) -> String {
        let image = image::RgbaImage::from_pixel(width, height, image::Rgba([shade, 0, 0, 255]));
        let mut bytes = Vec::new();
        image::DynamicImage::ImageRgba8(image)
            .write_to(&mut io::Cursor::new(&mut bytes), ImageFormat::Png)
            .unwrap();
        base64::engine::general_purpose::STANDARD.encode(bytes)
    }

    #[test]
    fn the_file_name_follows_the_data() {
        assert_eq!(preview_file_name("a"), preview_file_name("a"));
        assert_ne!(preview_file_name("a"), preview_file_name("b"));
        assert_eq!(preview_file_name("a").len(), 20);
    }

    #[test]
    fn a_large_png_is_fitted_into_the_thumbnail_box() {
        let dir = tempfile::tempdir().unwrap();
        let path = write_preview(dir.path(), &png_base64(2000, 1000, 1), "image/png").unwrap();
        assert_eq!(image::image_dimensions(path).unwrap(), (480, 240));
    }

    #[test]
    fn a_small_png_is_not_upscaled() {
        let dir = tempfile::tempdir().unwrap();
        let path = write_preview(dir.path(), &png_base64(40, 20, 1), "image/png").unwrap();
        assert_eq!(image::image_dimensions(path).unwrap(), (40, 20));
    }

    #[test]
    fn sync_keeps_the_current_set_and_deletes_the_rest() {
        let dir = tempfile::tempdir().unwrap();
        let png = |shade| (png_base64(10, 10, shade), "image/png".to_string());
        let (a, b, c) = (png(1), png(2), png(3));

        let first = sync_previews(dir.path(), &[a.clone(), b.clone()]).unwrap();
        assert_eq!(first.len(), 2);
        assert!(first.iter().all(|path| path.exists()));

        let second = sync_previews(dir.path(), &[b, c]).unwrap();
        assert!(!first[0].exists());
        assert_eq!(second[0], first[1]);
        assert!(second.iter().all(|path| path.exists()));
        assert_eq!(std::fs::read_dir(dir.path()).unwrap().count(), 2);
    }

    #[test]
    fn clearing_a_missing_dir_is_fine() {
        let dir = tempfile::tempdir().unwrap();
        clear_previews(&dir.path().join("missing"));
    }
}
