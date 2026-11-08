use crate::ports::{FileSystem, SidecarStore};
use std::path::Path;

use super::shared::{resolve_xmp_target, validate_key};

/// Sets an XMP property on an existing sidecar or its resolved basename.
///
/// Rejects invalid keys before writing and returns whether the property was
/// updated or already had the requested value.
pub fn set(
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

#[cfg(test)]
mod tests {
    use super::set;
    use crate::usecase::test_helpers::{StoreCall, TestFileSystem, TestSidecarStore};
    use rstest::rstest;
    use std::path::Path;

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
}
