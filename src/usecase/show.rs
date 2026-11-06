use crate::ports::{FileSystem, Selection, SidecarStore};
use std::path::Path;

use super::shared::{resolve_xmp_target, validate_key};

pub const DEFAULT_COMMENT: &str = "XMP-dc:Description";
pub(super) const BASIC_KEYS: &[&str] = &[
    "XMP-dc:Description",
    "XMP-dc:Creator",
    "XMP-dc:Rights",
    "XMP-xmp:CreateDate",
    "XMP-xmp:ModifyDate",
    "XMP-xmp:Rating",
    "XMP-photoshop:DateCreated",
];

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

#[cfg(test)]
mod tests {
    use super::{BASIC_KEYS, show};
    use crate::usecase::test_helpers::{StoreCall, TestFileSystem, TestSidecarStore};
    use rstest::rstest;
    use std::path::Path;

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
}
