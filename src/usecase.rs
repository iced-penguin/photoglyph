use crate::port::{Editor, FileSystem, Selection, SidecarStore};
use std::io::ErrorKind;
use std::path::{Path, PathBuf};

pub const DEFAULT_COMMENT: &str = "XMP-dc:Description";
const EXTENSIONS: &[&str] = &["jpg", "jpeg", "png", "nef", "tif", "tiff", "dng"];
const BASIC_KEYS: &[&str] = &[
    "XMP-dc:Description",
    "XMP-dc:Creator",
    "XMP-dc:Rights",
    "XMP-xmp:CreateDate",
    "XMP-xmp:ModifyDate",
    "XMP-xmp:Rating",
    "XMP-photoshop:DateCreated",
];
pub fn export(
    image: &Path,
    force: bool,
    tool: &impl SidecarStore,
    filesystem: &impl FileSystem,
) -> Result<String, String> {
    if !is_image(image) {
        return Err("export requires a supported image path".into());
    }
    let image = canonical(image, filesystem)?;
    if let Some(collision) = find_stem_collision(&image, filesystem)? {
        if force {
            eprintln!(
                "warning: sidecar basename is ambiguous with {}; proceeding because --force was specified",
                collision.display()
            );
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

pub fn show(
    target: &Path,
    all: bool,
    basic: bool,
    verbose: bool,
    key: Option<&str>,
    tool: &impl SidecarStore,
    filesystem: &impl FileSystem,
) -> Result<String, String> {
    let sidecar = resolve_xmp_target(target, filesystem)?;
    if all {
        return tool.read_properties(&sidecar, Selection::All, verbose);
    }
    let keys: Vec<&str> = match key {
        Some(key) => {
            validate_key(key)?;
            vec![key]
        }
        None if basic => BASIC_KEYS.to_vec(),
        None => vec![DEFAULT_COMMENT],
    };
    tool.read_properties(&sidecar, Selection::Keys(&keys), verbose)
}

pub fn set(
    target: &Path,
    key: &str,
    value: &str,
    tool: &impl SidecarStore,
    filesystem: &impl FileSystem,
) -> Result<String, String> {
    write_property(target, key, value, tool, filesystem)
}

pub fn clear(
    target: &Path,
    key: &str,
    tool: &impl SidecarStore,
    filesystem: &impl FileSystem,
) -> Result<String, String> {
    validate_key(key)?;
    let sidecar = resolve_xmp_target(target, filesystem)?;
    tool.clear_property(&sidecar, key)
        .map(|updated| if updated { "cleared" } else { "unchanged" }.into())
}

pub fn edit(
    target: &Path,
    editor: &impl Editor,
    filesystem: &impl FileSystem,
) -> Result<(), String> {
    let sidecar = resolve_xmp_target(target, filesystem)?;
    editor.edit(&sidecar)
}

fn write_property(
    target: &Path,
    key: &str,
    value: &str,
    tool: &impl SidecarStore,
    filesystem: &impl FileSystem,
) -> Result<String, String> {
    validate_key(key)?;
    let sidecar = resolve_xmp_target(target, filesystem)?;
    tool.write_property(&sidecar, key, value)
        .map(|updated| if updated { "updated" } else { "unchanged" }.into())
}

/// Accepts an .xmp path or a bare basename (`photo` means `photo.xmp`); image paths are rejected.
fn resolve_xmp_target(target: &Path, filesystem: &impl FileSystem) -> Result<PathBuf, String> {
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

fn canonical(path: &Path, filesystem: &impl FileSystem) -> Result<PathBuf, String> {
    filesystem
        .canonicalize(path)
        .map_err(|error| match error.kind() {
            ErrorKind::NotFound => format!("file not found: {}", path.display()),
            _ => error.to_string(),
        })
}

fn companion(image: &Path) -> PathBuf {
    image.with_extension("xmp")
}

fn find_stem_collision(
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

fn parent_or_current_dir(parent: Option<&Path>) -> &Path {
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

fn validate_key(key: &str) -> Result<(), String> {
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

fn is_image(path: &Path) -> bool {
    path.extension()
        .and_then(|ext| ext.to_str())
        .is_some_and(|ext| EXTENSIONS.contains(&ext.to_ascii_lowercase().as_str()))
}

fn is_xmp(path: &Path) -> bool {
    path.extension()
        .and_then(|ext| ext.to_str())
        .is_some_and(|ext| ext.eq_ignore_ascii_case("xmp"))
}

#[cfg(test)]
#[path = "usecase_test_helpers.rs"]
mod test_helpers;

#[cfg(test)]
mod tests {
    use super::test_helpers::{StoreCall, TestEditor, TestFileSystem, TestSidecarStore};
    use super::{
        BASIC_KEYS, clear, export, find_stem_collision, is_image, is_xmp, parent_or_current_dir,
        resolve_xmp_target, set, show, validate_key,
    };
    use crate::port::FileSystem;
    use std::io::ErrorKind;
    use std::path::Path;
    use std::path::PathBuf;

    use rstest::rstest;

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

        assert!(
            export(image, false, &store, &filesystem)
                .unwrap_err()
                .contains("basename is ambiguous")
        );
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
        assert!(export(image, true, &store, &filesystem).is_ok());
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
        let store = TestSidecarStore {
            filesystem: filesystem.clone(),
            export_creates_file: true,
            ..Default::default()
        };
        assert!(export(image, true, &store, &filesystem).is_ok());
    }

    #[test]
    fn show_uses_default_key() {
        let filesystem = TestFileSystem::default();
        let sidecar = Path::new("/photos/photo.xmp");
        filesystem.add_file(sidecar);
        let store = TestSidecarStore::default();
        assert_eq!(
            show(
                Path::new("/photos/photo"),
                false,
                false,
                true,
                None,
                &store,
                &filesystem
            )
            .unwrap(),
            "properties"
        );
        assert_eq!(
            store.calls.borrow().as_slice(),
            &[StoreCall::Read(
                sidecar.to_path_buf(),
                "XMP-dc:Description".into(),
                true
            )]
        );
    }

    #[test]
    fn show_propagates_read_errors() {
        let filesystem = TestFileSystem::default();
        let sidecar = Path::new("/photos/photo.xmp");
        filesystem.add_file(sidecar);
        let error_store = TestSidecarStore {
            read_error: Some("read failed".into()),
            ..Default::default()
        };
        assert_eq!(
            show(sidecar, true, false, false, None, &error_store, &filesystem).unwrap_err(),
            "read failed"
        );
    }

    #[rstest]
    #[case(true, true, false, None, "all", false)]
    #[case(true, false, true, None, "all", true)]
    #[case(false, true, false, None, BASIC_KEYS.join(","), false)]
    #[case(false, false, false, None, "XMP-dc:Description".to_string(), false)]
    #[case(false, false, true, Some("XMP-dc:Creator"), "XMP-dc:Creator".to_string(), true)]
    #[case(false, true, false, Some("XMP-dc:Creator"), "XMP-dc:Creator".to_string(), false)]
    fn show_selects_properties_to_read(
        #[case] all: bool,
        #[case] basic: bool,
        #[case] verbose: bool,
        #[case] key: Option<&str>,
        #[case] expected_selection: impl Into<String>,
        #[case] expected_verbose: bool,
    ) {
        let filesystem = TestFileSystem::default();
        let sidecar = Path::new("/photos/photo.xmp");
        filesystem.add_file(sidecar);
        let store = TestSidecarStore::default();

        assert!(show(sidecar, all, basic, verbose, key, &store, &filesystem).is_ok());
        assert_eq!(
            store.calls.borrow().as_slice(),
            &[StoreCall::Read(
                sidecar.into(),
                expected_selection.into(),
                expected_verbose
            )]
        );
    }

    #[test]
    fn show_reports_missing_sidecars_before_validating_keys() {
        let filesystem = TestFileSystem::default();
        let store = TestSidecarStore::default();
        assert!(
            show(
                Path::new("/photos/photo.xmp"),
                false,
                false,
                false,
                Some("bad key"),
                &store,
                &filesystem
            )
            .unwrap_err()
            .contains("sidecar not found")
        );
        assert!(store.calls.borrow().is_empty());
    }

    #[test]
    fn show_rejects_invalid_keys_for_existing_sidecars() {
        let filesystem = TestFileSystem::default();
        let store = TestSidecarStore::default();
        filesystem.add_file(Path::new("/photos/photo.xmp"));
        assert!(
            show(
                Path::new("/photos/photo.xmp"),
                false,
                false,
                false,
                Some("bad key"),
                &store,
                &filesystem
            )
            .unwrap_err()
            .contains("invalid XMP key")
        );
    }

    #[test]
    fn set_propagates_target_resolution_errors() {
        let filesystem = TestFileSystem::default();
        let store = TestSidecarStore::default();
        let missing = Path::new("/photos/missing.xmp");
        assert!(
            set(missing, "XMP-dc:Description", "value", &store, &filesystem)
                .unwrap_err()
                .contains("sidecar not found")
        );
    }

    #[test]
    fn clear_propagates_target_resolution_errors() {
        let filesystem = TestFileSystem::default();
        let store = TestSidecarStore::default();
        let missing = Path::new("/photos/missing.xmp");
        assert!(
            clear(missing, "XMP-dc:Description", &store, &filesystem)
                .unwrap_err()
                .contains("sidecar not found")
        );
    }

    #[test]
    fn edit_propagates_target_resolution_errors() {
        let filesystem = TestFileSystem::default();
        let missing = Path::new("/photos/missing.xmp");
        assert!(
            super::edit(missing, &TestEditor::default(), &filesystem)
                .unwrap_err()
                .contains("sidecar not found")
        );
    }

    #[rstest]
    #[case(None, Ok("updated"))]
    #[case(Some(Ok(true)), Ok("updated"))]
    #[case(Some(Ok(false)), Ok("unchanged"))]
    #[case(Some(Err("write failed".to_string())), Err("write failed"))]
    fn set_reports_store_outcome(
        #[case] write_result: Option<Result<bool, String>>,
        #[case] expected: Result<&str, &str>,
    ) {
        let filesystem = TestFileSystem::default();
        let sidecar = Path::new("/photos/photo.xmp");
        filesystem.add_file(sidecar);
        let store = TestSidecarStore {
            write_result,
            ..Default::default()
        };

        let result = set(sidecar, "XMP-dc:Description", "hello", &store, &filesystem);

        assert_eq!(result, expected.map(String::from).map_err(String::from));
        assert_eq!(
            store.calls.borrow().as_slice(),
            &[StoreCall::Write(
                sidecar.to_path_buf(),
                "XMP-dc:Description".into(),
                "hello".into()
            )]
        );
    }

    #[rstest]
    #[case(None, Ok("cleared"))]
    #[case(Some(Ok(true)), Ok("cleared"))]
    #[case(Some(Ok(false)), Ok("unchanged"))]
    #[case(Some(Err("clear failed".to_string())), Err("clear failed"))]
    fn clear_reports_store_outcome(
        #[case] clear_result: Option<Result<bool, String>>,
        #[case] expected: Result<&str, &str>,
    ) {
        let filesystem = TestFileSystem::default();
        let sidecar = Path::new("/photos/photo.xmp");
        filesystem.add_file(sidecar);
        let store = TestSidecarStore {
            clear_result,
            ..Default::default()
        };

        let result = clear(sidecar, "XMP-dc:Description", &store, &filesystem);

        assert_eq!(result, expected.map(String::from).map_err(String::from));
        assert_eq!(
            store.calls.borrow().as_slice(),
            &[StoreCall::Clear(
                sidecar.to_path_buf(),
                "XMP-dc:Description".into()
            )]
        );
    }

    #[rstest]
    #[case("bad key")]
    #[case("XMP-dc:Description=value")]
    fn set_rejects_invalid_keys_without_touching_the_store(#[case] key: &str) {
        let filesystem = TestFileSystem::default();
        let sidecar = Path::new("/photos/photo.xmp");
        filesystem.add_file(sidecar);
        let store = TestSidecarStore::default();

        assert!(
            set(sidecar, key, "hello", &store, &filesystem)
                .unwrap_err()
                .contains("invalid XMP key")
        );
        assert!(store.calls.borrow().is_empty());
    }

    #[rstest]
    #[case("bad key")]
    #[case("XMP-dc:Description=value")]
    fn clear_rejects_invalid_keys_without_touching_the_store(#[case] key: &str) {
        let filesystem = TestFileSystem::default();
        let sidecar = Path::new("/photos/photo.xmp");
        filesystem.add_file(sidecar);
        let store = TestSidecarStore::default();
        assert!(
            clear(sidecar, key, &store, &filesystem)
                .unwrap_err()
                .contains("invalid XMP key")
        );
        assert!(store.calls.borrow().is_empty());
    }

    #[test]
    fn edit_opens_the_resolved_sidecar_in_the_editor() {
        let filesystem = TestFileSystem::default();
        let sidecar = Path::new("/photos/photo.xmp");
        filesystem.add_file(sidecar);
        let editor = TestEditor::default();
        assert_eq!(super::edit(sidecar, &editor, &filesystem), Ok(()));
        assert_eq!(*editor.calls.borrow(), vec![sidecar.to_path_buf()]);
    }

    #[test]
    fn edit_propagates_editor_errors() {
        let filesystem = TestFileSystem::default();
        let sidecar = Path::new("/photos/photo.xmp");
        filesystem.add_file(sidecar);
        let editor = TestEditor {
            error: Some("editor failed".into()),
            ..Default::default()
        };
        assert_eq!(
            super::edit(sidecar, &editor, &filesystem),
            Err("editor failed".into())
        );
    }
}
