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

#[cfg(test)]
mod tests {
    use super::{is_image, is_xmp, resolve_xmp_target, validate_key};
    use crate::usecase::test_helpers::TestFileSystem;
    use rstest::rstest;
    use std::path::{Path, PathBuf};

    #[rstest]
    #[case("photo.jpg", true)]
    #[case("photo.jpeg", true)]
    #[case("photo.png", true)]
    #[case("photo.nef", true)]
    #[case("photo.tif", true)]
    #[case("photo.tiff", true)]
    #[case("photo.dng", true)]
    #[case("photo.NEF", true)]
    #[case("photo.xmp", false)]
    #[case("photo.txt", false)]
    #[case("photo", false)]
    fn detects_supported_image_extensions(#[case] path: &str, #[case] expected: bool) {
        assert_eq!(is_image(Path::new(path)), expected);
    }

    #[rstest]
    #[case("photo.xmp", true)]
    #[case("photo.XMP", true)]
    #[case("photo.jpg", false)]
    #[case("photo", false)]
    fn detects_xmp_extensions(#[case] path: &str, #[case] expected: bool) {
        assert_eq!(is_xmp(Path::new(path)), expected);
    }

    #[test]
    fn rejects_image_paths_for_xmp_targets() {
        assert!(
            resolve_xmp_target(Path::new("photo.jpg"), &TestFileSystem::default())
                .unwrap_err()
                .contains("expected an .xmp file or a basename")
        );
    }

    #[rstest]
    #[case("XMP-dc:Description", true)]
    #[case("XMP-xmp:Rating", true)]
    #[case("XMP-dc:Description=value", false)]
    #[case("-XMP-dc:Description", false)]
    #[case("XMP-dc:Description\n-other", false)]
    #[case("bad key", false)]
    #[case("Description", false)]
    #[case("", false)]
    #[case("XMP-dc:Descripción", false)]
    fn validates_exiftool_keys_as_individual_arguments(#[case] key: &str, #[case] valid: bool) {
        assert_eq!(validate_key(key).is_ok(), valid);
    }
    #[test]
    fn rejects_non_xmp_canonical_targets() {
        let filesystem = TestFileSystem::default();
        let sidecar = Path::new("/photos/photo.xmp");
        filesystem.add_file(sidecar);
        filesystem.0.borrow_mut().canonicalize_to = Some(PathBuf::from("/photos/resolved.txt"));
        assert!(
            resolve_xmp_target(sidecar, &filesystem)
                .unwrap_err()
                .contains("not an .xmp sidecar")
        );
    }
}
