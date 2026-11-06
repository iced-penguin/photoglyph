use crate::ports::FileSystem;
use std::io::ErrorKind;
use std::path::{Path, PathBuf};

const EXTENSIONS: &[&str] = &["jpg", "jpeg", "png", "nef", "tif", "tiff", "dng"];

/// Accepts an .xmp path or a bare basename (`photo` means `photo.xmp`); image paths are rejected.
pub(super) fn resolve_xmp_target(
    target: &Path,
    filesystem: &impl FileSystem,
) -> Result<PathBuf, String> {
    if is_image(target) {
        return Err("expected an .xmp file or a basename (e.g. `photo` for photo.xmp)".into());
    }
    let path = if is_xmp(target) {
        target.to_path_buf()
    } else {
        let mut name = target.as_os_str().to_os_string();
        name.push(".xmp");
        PathBuf::from(name)
    };
    if !filesystem.is_file(&path) {
        return Err(format!("sidecar not found: {}", path.display()));
    }
    canonical(&path, filesystem).and_then(require_xmp)
}

fn require_xmp(path: PathBuf) -> Result<PathBuf, String> {
    if is_xmp(&path) {
        Ok(path)
    } else {
        Err("resolved file is not an .xmp sidecar".into())
    }
}

pub(super) fn canonical(path: &Path, filesystem: &impl FileSystem) -> Result<PathBuf, String> {
    filesystem
        .canonicalize(path)
        .map_err(|error| match error.kind() {
            ErrorKind::NotFound => format!("file not found: {}", path.display()),
            _ => error.to_string(),
        })
}

pub(super) fn is_image(path: &Path) -> bool {
    path.extension()
        .and_then(|ext| ext.to_str())
        .is_some_and(|ext| EXTENSIONS.contains(&ext.to_ascii_lowercase().as_str()))
}

pub(super) fn validate_key(key: &str) -> Result<(), String> {
    if key.is_empty()
        || key.starts_with('-')
        || !key.contains(':')
        || !key
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b == b':' || b == b'_' || b == b'-')
    {
        return Err("invalid XMP key; use ExifTool syntax such as XMP-dc:Description".into());
    }
    Ok(())
}

pub(super) fn is_xmp(path: &Path) -> bool {
    path.extension()
        .and_then(|ext| ext.to_str())
        .is_some_and(|ext| ext.eq_ignore_ascii_case("xmp"))
}
