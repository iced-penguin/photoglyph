use crate::ports::{FileSystem, Logger, SidecarStore};
use std::path::{Path, PathBuf};

use super::shared::{canonical, is_image};

/// Exports image metadata to its companion XMP sidecar.
///
/// The image must be an existing supported image file. An existing sidecar is
/// replaced only when `force` is enabled; basename collisions are rejected
/// unless forced, in which case a warning is reported.
///
/// Returns a user-facing success message or an error.
pub fn export(
    image: &Path,
    force: bool,
    tool: &impl SidecarStore,
    filesystem: &impl FileSystem,
    logger: &mut impl Logger,
) -> Result<String, String> {
    if !is_image(image) {
        return Err("export requires a supported image path".into());
    }
    let image = canonical(image, filesystem)?;
    if let Some(collision) = find_stem_collision(&image, filesystem)? {
        if force {
            logger.warn(&format!(
                "sidecar basename is ambiguous with {}; proceeding because --force was specified",
                collision.display()
            ));
        } else {
            return Err(stem_collision_error(&collision));
        }
    }
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

pub(super) fn find_stem_collision(
    image: &Path,
    filesystem: &impl FileSystem,
) -> Result<Option<PathBuf>, String> {
    let Some(stem) = image.file_stem() else {
        return Err("image path has no filename".into());
    };
    let parent = parent_or_current_dir(image.parent());
    let entries = filesystem
        .read_dir(parent)
        .map_err(|error| error.to_string())?;
    for path in entries {
        if path == image || !is_image(&path) {
            continue;
        }
        if path.file_stem().is_some_and(|other| other == stem) {
            return Ok(Some(path));
        }
    }
    Ok(None)
}

pub(super) fn parent_or_current_dir(parent: Option<&Path>) -> &Path {
    parent.unwrap_or_else(|| Path::new("."))
}

fn stem_collision_error(collision: &Path) -> String {
    format!(
        "sidecar basename is ambiguous with {}; refusing unsafe operation",
        collision.display()
    )
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
    use super::{find_stem_collision, parent_or_current_dir};
    use crate::ports::{FileSystem, Logger};
    use crate::usecase::test_helpers::{StoreCall, TestFileSystem, TestSidecarStore};
    use rstest::rstest;
    use std::cell::RefCell;
    use std::io::ErrorKind;
    use std::path::{Path, PathBuf};
    use std::rc::Rc;

    #[derive(Default)]
    struct TestLogger {
        warnings: Vec<String>,
        events: Option<Rc<RefCell<Vec<String>>>>,
    }

    impl Logger for TestLogger {
        fn warn(&mut self, message: &str) {
            self.warnings.push(message.into());
            if let Some(events) = &self.events {
                events.borrow_mut().push("warning".into());
            }
        }
    }

    fn export(
        image: &Path,
        force: bool,
        store: &TestSidecarStore,
        filesystem: &TestFileSystem,
    ) -> Result<String, String> {
        super::export(image, force, store, filesystem, &mut TestLogger::default())
    }

    #[test]
    fn finds_an_image_with_the_same_basename() {
        let filesystem = TestFileSystem::default();
        let jpeg = Path::new("/photos/photo.jpg");
        filesystem.add_file(jpeg);
        filesystem.add_file(Path::new("/photos/photo.NEF"));
        assert_eq!(
            find_stem_collision(jpeg, &filesystem).unwrap(),
            Some(PathBuf::from("/photos/photo.NEF"))
        );
    }

    #[test]
    fn does_not_find_a_collision_for_a_different_basename() {
        let filesystem = TestFileSystem::default();
        let jpeg = Path::new("/photos/photo.jpg");
        filesystem.add_file(jpeg);
        filesystem.add_file(Path::new("/photos/other.jpg"));
        assert_eq!(find_stem_collision(jpeg, &filesystem).unwrap(), None);
    }

    #[test]
    fn reports_missing_image_filename_for_stem_collision_checks() {
        assert!(
            find_stem_collision(Path::new("/"), &TestFileSystem::default())
                .unwrap_err()
                .contains("no filename")
        );
    }

    #[rstest]
    #[case(None, ".")]
    #[case(Some("/photos"), "/photos")]
    fn resolves_image_parent_or_defaults_to_current_directory(
        #[case] parent: Option<&str>,
        #[case] expected: &str,
    ) {
        assert_eq!(
            parent_or_current_dir(parent.map(Path::new)),
            Path::new(expected)
        );
    }

    #[test]
    fn export_rejects_unsupported_image_paths() {
        let filesystem = TestFileSystem::default();
        let store = TestSidecarStore::default();
        assert_eq!(
            export(Path::new("/photos/photo.txt"), false, &store, &filesystem).unwrap_err(),
            "export requires a supported image path"
        );
        assert!(store.calls.borrow().is_empty());
    }

    #[test]
    fn export_reports_missing_images() {
        let filesystem = TestFileSystem::default();
        let store = TestSidecarStore::default();
        assert!(
            export(Path::new("/photos/missing.jpg"), false, &store, &filesystem)
                .unwrap_err()
                .contains("file not found")
        );
        assert!(store.calls.borrow().is_empty());
    }

    #[test]
    fn export_reports_image_canonicalization_errors() {
        let filesystem = TestFileSystem::default();
        let store = TestSidecarStore::default();
        filesystem.add_file(Path::new("/photos/photo.jpg"));
        filesystem.0.borrow_mut().canonicalize_error = Some(ErrorKind::PermissionDenied);
        assert!(
            export(Path::new("/photos/photo.jpg"), false, &store, &filesystem)
                .unwrap_err()
                .contains("denied")
        );
        assert!(store.calls.borrow().is_empty());
    }

    #[test]
    fn export_reports_directory_read_errors() {
        let filesystem = TestFileSystem::default();
        let image = Path::new("/photos/photo.jpg");
        filesystem.add_file(image);
        filesystem.0.borrow_mut().read_dir_error = Some(ErrorKind::PermissionDenied);
        let store = TestSidecarStore::default();
        assert!(
            export(image, false, &store, &filesystem)
                .unwrap_err()
                .contains("denied")
        );
        assert!(store.calls.borrow().is_empty());
    }

    #[test]
    fn export_reports_missing_output_after_tool_success() {
        let filesystem = TestFileSystem::default();
        let image = Path::new("/photos/photo.jpg");
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
    fn export_rejects_image_basename_collisions_without_force() {
        let filesystem = TestFileSystem::default();
        let image = Path::new("/photos/photo.jpg");
        filesystem.add_file(image);
        filesystem.add_file(Path::new("/photos/photo.NEF"));
        let store = TestSidecarStore::default();

        let mut logger = TestLogger::default();
        assert_eq!(
            super::export(image, false, &store, &filesystem, &mut logger).unwrap_err(),
            "sidecar basename is ambiguous with /photos/photo.NEF; refusing unsafe operation"
        );
        assert!(logger.warnings.is_empty());
        assert!(store.calls.borrow().is_empty());
    }

    #[test]
    fn export_with_force_replaces_existing_sidecars() {
        let filesystem = TestFileSystem::default();
        let image = Path::new("/photos/photo.jpg");
        let sidecar = Path::new("/photos/photo.xmp");
        filesystem.add_file(image);
        filesystem.add_file(sidecar);
        let store = TestSidecarStore {
            filesystem: filesystem.clone(),
            export_creates_file: true,
            ..Default::default()
        };
        let mut logger = TestLogger::default();
        assert!(super::export(image, true, &store, &filesystem, &mut logger).is_ok());
        assert!(logger.warnings.is_empty());
        assert!(filesystem.is_file(sidecar));
        assert_eq!(filesystem.file_count(), 2);
    }

    #[test]
    fn export_reports_sidecars_created_during_install_without_force() {
        let filesystem = TestFileSystem::default();
        let image = Path::new("/photos/photo.jpg");
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
        let image = Path::new("/photos/photo.jpg");
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
        let image = Path::new("/photos/photo.jpg");
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
        let image = Path::new("/photos/photo.jpg");
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
        let image = Path::new("/photos/photo.jpg");
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
        let image = Path::new("/photos/photo.jpg");
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
        let image = Path::new("/photos/photo.jpg");
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
    fn forced_export_warns_when_another_image_shares_its_basename() {
        let filesystem = TestFileSystem::default();
        let image = Path::new("/photos/photo.jpg");
        filesystem.add_file(image);
        filesystem.add_file(Path::new("/photos/photo.NEF"));
        let events = Rc::new(RefCell::new(Vec::new()));
        let store = TestSidecarStore {
            filesystem: filesystem.clone(),
            export_creates_file: true,
            events: Some(events.clone()),
            ..Default::default()
        };
        let mut logger = TestLogger {
            events: Some(events.clone()),
            ..Default::default()
        };
        assert_eq!(
            super::export(image, true, &store, &filesystem, &mut logger).unwrap(),
            "exported to /photos/photo.xmp"
        );
        assert_eq!(
            logger.warnings,
            [
                "sidecar basename is ambiguous with /photos/photo.NEF; proceeding because --force was specified"
            ]
        );
        assert_eq!(*events.borrow(), ["warning", "export"]);
    }

    #[rstest]
    #[case::export_failure(true)]
    #[case::install_failure(false)]
    fn forced_export_preserves_warning_on_failure(#[case] export_failure: bool) {
        let filesystem = TestFileSystem::default();
        let image = Path::new("/photos/photo.jpg");
        filesystem.add_file(image);
        filesystem.add_file(Path::new("/photos/photo.NEF"));
        if !export_failure {
            filesystem.0.borrow_mut().install_error = Some(ErrorKind::PermissionDenied);
        }
        let store = TestSidecarStore {
            filesystem: filesystem.clone(),
            export_creates_file: true,
            export_error: export_failure.then(|| "export failed".into()),
            ..Default::default()
        };
        let mut logger = TestLogger::default();
        let error = super::export(image, true, &store, &filesystem, &mut logger).unwrap_err();
        if export_failure {
            assert_eq!(error, "export failed");
        } else {
            assert!(error.contains("cannot install sidecar /photos/photo.xmp:"));
        }
        assert_eq!(
            logger.warnings,
            [
                "sidecar basename is ambiguous with /photos/photo.NEF; proceeding because --force was specified"
            ]
        );
        assert_eq!(filesystem.file_count(), 2);
    }
}
