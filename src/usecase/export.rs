use crate::ports::{FileSystem, SidecarStore};
use std::path::{Path, PathBuf};

use super::shared::{canonical, is_raw};

/// Exports image metadata to its companion XMP sidecar.
///
/// The image must be an existing file with a supported RAW extension. An
/// existing sidecar is replaced only when `force` is enabled.
///
/// Returns a user-facing success message or an error.
pub fn export(
    image: &Path,
    force: bool,
    tool: &impl SidecarStore,
    filesystem: &impl FileSystem,
) -> Result<String, String> {
    if !is_raw(image) {
        return Err("export requires a supported RAW image path".into());
    }
    let image = canonical(image, filesystem)?;
    let sidecar = companion(&image);
    if filesystem.exists(&sidecar) && !force {
        return Err(format!(
            "sidecar already exists: {}; use --force to replace it",
            sidecar.display()
        ));
    }
    let temporary = temporary_sidecar(&sidecar, filesystem);
    if let Err(error) = tool.export_to(&image, &temporary) {
        let _ = filesystem.remove_file(&temporary);
        return Err(error);
    }
    if !filesystem.is_file(&temporary) {
        return Err("exiftool reported success but did not create an XMP sidecar".into());
    }
    if let Err(error) = filesystem.install_sidecar(&temporary, &sidecar, force) {
        let _ = filesystem.remove_file(&temporary);
        return Err(if filesystem.exists(&sidecar) && !force {
            format!(
                "sidecar already exists: {}; use --force to replace it",
                sidecar.display()
            )
        } else {
            format!("cannot install sidecar {}: {error}", sidecar.display())
        });
    }
    Ok(format!("exported to {}", sidecar.display()))
}

fn companion(image: &Path) -> PathBuf {
    image.with_extension("xmp")
}

fn temporary_sidecar(sidecar: &Path, filesystem: &impl FileSystem) -> PathBuf {
    let name = sidecar.file_name().unwrap_or_default().to_string_lossy();
    let mut index = 0u32;
    loop {
        let candidate =
            sidecar.with_file_name(format!(".{name}.{}.{}.tmp.xmp", std::process::id(), index));
        if !filesystem.exists(&candidate) {
            return candidate;
        }
        index += 1;
    }
}

#[cfg(test)]
mod tests {
    use crate::ports::FileSystem;
    use crate::usecase::test_helpers::{StoreCall, TestFileSystem, TestSidecarStore};
    use rstest::rstest;
    use std::io::ErrorKind;
    use std::path::{Path, PathBuf};

    fn export(
        image: &Path,
        force: bool,
        store: &TestSidecarStore,
        filesystem: &TestFileSystem,
    ) -> Result<String, String> {
        super::export(image, force, store, filesystem)
    }

    #[rstest]
    #[case("photo.jpg")]
    #[case("photo.jpeg")]
    #[case("photo.png")]
    #[case("photo.tiff")]
    #[case("photo.mov")]
    #[case("photo.txt")]
    #[case("photo")]
    fn export_rejects_non_raw_paths_without_calling_the_tool(#[case] path: &str) {
        let filesystem = TestFileSystem::default();
        let store = TestSidecarStore::default();
        assert_eq!(
            export(Path::new(path), false, &store, &filesystem).unwrap_err(),
            "export requires a supported RAW image path"
        );
        assert!(store.calls.borrow().is_empty());
    }

    #[test]
    fn export_reports_missing_images() {
        let filesystem = TestFileSystem::default();
        let store = TestSidecarStore::default();
        assert!(
            export(Path::new("/photos/missing.nef"), false, &store, &filesystem)
                .unwrap_err()
                .contains("file not found")
        );
        assert!(store.calls.borrow().is_empty());
    }

    #[test]
    fn export_reports_image_canonicalization_errors() {
        let filesystem = TestFileSystem::default();
        let store = TestSidecarStore::default();
        filesystem.add_file(Path::new("/photos/photo.nef"));
        filesystem.0.borrow_mut().canonicalize_error = Some(ErrorKind::PermissionDenied);
        assert!(
            export(Path::new("/photos/photo.nef"), false, &store, &filesystem)
                .unwrap_err()
                .contains("denied")
        );
        assert!(store.calls.borrow().is_empty());
    }

    #[test]
    fn export_reports_missing_output_after_tool_success() {
        let filesystem = TestFileSystem::default();
        let image = Path::new("/photos/photo.nef");
        filesystem.add_file(image);
        let store = TestSidecarStore {
            filesystem: filesystem.clone(),
            ..Default::default()
        };
        assert!(
            export(image, false, &store, &filesystem)
                .unwrap_err()
                .contains("did not create an XMP sidecar")
        );
        assert_eq!(filesystem.file_count(), 1);
    }

    #[test]
    fn export_with_force_replaces_existing_sidecars() {
        let filesystem = TestFileSystem::default();
        let image = Path::new("/photos/photo.nef");
        let sidecar = Path::new("/photos/photo.xmp");
        filesystem.add_file(image);
        filesystem.add_file(sidecar);
        let store = TestSidecarStore {
            filesystem: filesystem.clone(),
            export_creates_file: true,
            ..Default::default()
        };
        assert!(export(image, true, &store, &filesystem).is_ok());
        assert!(filesystem.is_file(sidecar));
        assert_eq!(filesystem.file_count(), 2);
    }

    #[test]
    fn export_reports_sidecars_created_during_install_without_force() {
        let filesystem = TestFileSystem::default();
        let image = Path::new("/photos/photo.nef");
        filesystem.add_file(image);
        {
            let mut state = filesystem.0.borrow_mut();
            state.install_error = Some(ErrorKind::AlreadyExists);
            state.create_sidecar_during_install = true;
        }
        let store = TestSidecarStore {
            filesystem: filesystem.clone(),
            export_creates_file: true,
            ..Default::default()
        };
        assert!(
            export(image, false, &store, &filesystem)
                .unwrap_err()
                .contains("use --force to replace it")
        );
        assert!(filesystem.is_file(Path::new("/photos/photo.xmp")));
    }

    #[test]
    fn export_skips_existing_temporary_names() {
        let filesystem = TestFileSystem::default();
        let image = Path::new("/photos/photo.nef");
        filesystem.add_file(image);
        let existing_temporary = PathBuf::from(format!(
            "/photos/.photo.xmp.{}.0.tmp.xmp",
            std::process::id()
        ));
        filesystem.add_file(&existing_temporary);
        let store = TestSidecarStore {
            filesystem: filesystem.clone(),
            export_creates_file: true,
            ..Default::default()
        };

        assert!(export(image, false, &store, &filesystem).is_ok());
        assert!(filesystem.is_file(&existing_temporary));
        assert!(matches!(
            store.calls.borrow().as_slice(),
            [StoreCall::Export(_, temporary)] if temporary != &existing_temporary
        ));
    }

    #[test]
    fn export_installs_a_created_sidecar() {
        let filesystem = TestFileSystem::default();
        let image = Path::new("/photos/photo.nef");
        let sidecar = Path::new("/photos/photo.xmp");
        filesystem.add_file(image);
        let store = TestSidecarStore {
            filesystem: filesystem.clone(),
            export_creates_file: true,
            ..Default::default()
        };

        assert_eq!(
            export(image, false, &store, &filesystem).unwrap(),
            "exported to /photos/photo.xmp"
        );
        assert!(filesystem.is_file(sidecar));
        assert_eq!(filesystem.file_count(), 2);
        assert!(matches!(
            store.calls.borrow().as_slice(),
            [StoreCall::Export(called_image, temporary)]
                if called_image == image && temporary != sidecar
        ));
    }

    #[test]
    fn export_rejects_existing_sidecars_and_preserves_them_without_force() {
        let filesystem = TestFileSystem::default();
        let image = Path::new("/photos/photo.nef");
        let sidecar = Path::new("/photos/photo.xmp");
        filesystem.add_file(image);
        filesystem.add_file(sidecar);
        let store = TestSidecarStore {
            filesystem: filesystem.clone(),
            ..Default::default()
        };

        assert!(
            export(image, false, &store, &filesystem)
                .unwrap_err()
                .contains("use --force to replace it")
        );
        assert!(store.calls.borrow().is_empty());
        assert!(filesystem.is_file(sidecar));
    }

    #[test]
    fn export_cleans_up_temporary_sidecar_after_tool_failure() {
        let image = Path::new("/photos/photo.nef");
        let filesystem = TestFileSystem::default();
        filesystem.add_file(image);
        let tool_error = TestSidecarStore {
            filesystem: filesystem.clone(),
            export_error: Some("export failed".into()),
            export_creates_file: true,
            ..Default::default()
        };
        assert_eq!(
            export(image, false, &tool_error, &filesystem).unwrap_err(),
            "export failed"
        );
        assert_eq!(filesystem.file_count(), 1);
    }

    #[test]
    fn export_cleans_up_temporary_sidecar_after_install_failure() {
        let image = Path::new("/photos/photo.nef");
        let install_filesystem = TestFileSystem::default();
        install_filesystem.add_file(image);
        install_filesystem.0.borrow_mut().install_error = Some(ErrorKind::PermissionDenied);
        let store = TestSidecarStore {
            filesystem: install_filesystem.clone(),
            export_creates_file: true,
            ..Default::default()
        };
        assert!(
            export(image, false, &store, &install_filesystem)
                .unwrap_err()
                .contains("cannot install sidecar")
        );
        assert_eq!(install_filesystem.file_count(), 1);
    }

    #[test]
    fn export_propagates_tool_error_when_no_temporary_sidecar_was_created() {
        let image = Path::new("/photos/photo.nef");
        let missing_temporary_fs = TestFileSystem::default();
        missing_temporary_fs.add_file(image);
        let store = TestSidecarStore {
            export_error: Some("export failed before creating output".into()),
            ..Default::default()
        };
        assert_eq!(
            export(image, false, &store, &missing_temporary_fs).unwrap_err(),
            "export failed before creating output"
        );
        assert_eq!(missing_temporary_fs.file_count(), 1);
    }

    #[test]
    fn export_allows_same_basename_images_with_any_extension() {
        let filesystem = TestFileSystem::default();
        let image = Path::new("/photos/photo.nef");
        filesystem.add_file(image);
        filesystem.add_file(Path::new("/photos/photo.jpg"));
        filesystem.add_file(Path::new("/photos/photo.CR3"));
        let store = TestSidecarStore {
            filesystem: filesystem.clone(),
            export_creates_file: true,
            ..Default::default()
        };
        assert_eq!(
            export(image, false, &store, &filesystem).unwrap(),
            "exported to /photos/photo.xmp"
        );
        assert_eq!(store.calls.borrow().len(), 1);
        assert!(filesystem.is_file(Path::new("/photos/photo.jpg")));
        assert!(filesystem.is_file(Path::new("/photos/photo.CR3")));
    }
}
