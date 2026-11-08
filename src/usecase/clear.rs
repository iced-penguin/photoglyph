use crate::ports::{FileSystem, SidecarStore};
use std::path::Path;

use super::shared::{resolve_xmp_target, validate_key};

/// Clears an XMP property on an existing sidecar or its resolved basename.
///
/// Rejects invalid keys before writing and returns whether the property was
/// cleared or was already empty.
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

#[cfg(test)]
mod tests {
    use super::clear;
    use crate::usecase::test_helpers::{StoreCall, TestFileSystem, TestSidecarStore};
    use rstest::rstest;
    use std::path::Path;

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
}
