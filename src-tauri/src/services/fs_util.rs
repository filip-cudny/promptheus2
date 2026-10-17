use std::io;
use std::path::Path;

pub fn write_atomic(path: &Path, content: &[u8]) -> io::Result<()> {
    let (Some(parent), Some(file_name)) = (path.parent(), path.file_name()) else {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            format!("path has no parent or file name: {}", path.display()),
        ));
    };
    let tmp = parent.join(format!(".{}.tmp", file_name.to_string_lossy()));
    std::fs::write(&tmp, content)?;
    std::fs::rename(&tmp, path)
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn replaces_content_and_leaves_no_temp_file() {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("a.md");
        std::fs::write(&path, "old").unwrap();
        write_atomic(&path, b"new").unwrap();
        assert_eq!(std::fs::read_to_string(&path).unwrap(), "new");
        assert_eq!(std::fs::read_dir(dir.path()).unwrap().count(), 1);
    }

    #[test]
    fn rejects_path_without_parent() {
        assert!(write_atomic(Path::new("/"), b"x").is_err());
    }
}
