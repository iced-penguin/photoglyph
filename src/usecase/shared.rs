use crate::ports::FileSystem;
use std::io::ErrorKind;
use std::path::{Path, PathBuf};

const RAW_EXTENSIONS: &[&str] = &[
    "dng", "cr2", "cr3", "crw", "nef", "nrw", "arw", "arq", "sr2", "srf", "raf", "orf", "rw2",
    "rwl", "pef", "3fr", "fff", "iiq", "mos", "mef", "mrw", "srw", "x3f", "dcr", "k25", "kdc",
    "erf", "gpr", "raw",
];
const IMAGE_EXTENSIONS: &[&str] = &["jpg", "jpeg", "png", "tif", "tiff"];

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
    has_extension(path, IMAGE_EXTENSIONS) || is_raw(path)
}

pub(super) fn is_raw(path: &Path) -> bool {
    has_extension(path, RAW_EXTENSIONS)
}

fn has_extension(path: &Path, extensions: &[&str]) -> bool {
    path.extension()
        .and_then(|ext| ext.to_str())
        .is_some_and(|ext| {
            extensions
                .iter()
                .any(|supported| ext.eq_ignore_ascii_case(supported))
        })
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
    use super::{is_image, is_raw, is_xmp, resolve_xmp_target, validate_key};
    use crate::usecase::test_helpers::TestFileSystem;
    use rstest::rstest;
    use std::path::{Path, PathBuf};

    #[rstest]
    #[case("photo.jpg", true)]
    #[case("photo.jpeg", true)]
    #[case("photo.png", true)]
    #[case("photo.tif", true)]
    #[case("photo.tiff", true)]
    #[case("photo.nef", true)]
    #[case("photo.3fr", true)]
    #[case("photo.NEF", true)]
    #[case("photo.xmp", false)]
    #[case("photo.txt", false)]
    #[case("photo", false)]
    fn detects_supported_image_extensions(#[case] path: &str, #[case] expected: bool) {
        assert_eq!(is_image(Path::new(path)), expected);
    }

    #[rstest]
    #[case("dng")]
    #[case("cr2")]
    #[case("cr3")]
    #[case("crw")]
    #[case("nef")]
    #[case("nrw")]
    #[case("arw")]
    #[case("arq")]
    #[case("sr2")]
    #[case("srf")]
    #[case("raf")]
    #[case("orf")]
    #[case("rw2")]
    #[case("rwl")]
    #[case("pef")]
    #[case("3fr")]
    #[case("fff")]
    #[case("iiq")]
    #[case("mos")]
    #[case("mef")]
    #[case("mrw")]
    #[case("srw")]
    #[case("x3f")]
    #[case("dcr")]
    #[case("k25")]
    #[case("kdc")]
    #[case("erf")]
    #[case("gpr")]
    #[case("raw")]
    fn recognizes_every_supported_raw_extension(#[case] extension: &str) {
        assert!(is_raw(Path::new(&format!("photo.{extension}"))));
    }

    #[rstest]
    #[case("photo.DNG")]
    #[case("photo.Cr3")]
    #[case("photo.NEF")]
    #[case("photo.RaW")]
    fn recognizes_raw_extensions_case_insensitively(#[case] path: &str) {
        assert!(is_raw(Path::new(path)));
    }

    #[rstest]
    #[case("photo.jpg")]
    #[case("photo.jpeg")]
    #[case("photo.png")]
    #[case("photo.tif")]
    #[case("photo.tiff")]
    #[case("photo.mov")]
    #[case("photo.mp4")]
    #[case("photo.txt")]
    #[case("photo")]
    fn rejects_non_raw_extensions_for_export(#[case] path: &str) {
        assert!(!is_raw(Path::new(path)));
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
    #[case("photo.jpg")]
    #[case("photo.tiff")]
    #[case("photo.NEF")]
    #[case("photo.3fr")]
    fn rejects_old_and_new_image_paths_for_xmp_targets(#[case] target: &str) {
        assert!(
            resolve_xmp_target(Path::new(target), &TestFileSystem::default())
                .unwrap_err()
                .contains("expected an .xmp file or a basename")
        );
    }

    #[test]
    fn resolves_a_sidecar_without_a_source_image() {
        let filesystem = TestFileSystem::default();
        let sidecar = Path::new("/photos/photo.xmp");
        filesystem.add_file(sidecar);
        assert_eq!(resolve_xmp_target(sidecar, &filesystem).unwrap(), sidecar);
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
