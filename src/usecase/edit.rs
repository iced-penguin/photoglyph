use crate::ports::{Editor, FileSystem};
use std::path::Path;

use super::shared::resolve_xmp_target;

/// Opens an existing sidecar or its resolved basename in the configured editor.
///
/// Returns an error if the target cannot be resolved or the editor fails.
pub fn edit(
    target: &Path,
    editor: &impl Editor,
    filesystem: &impl FileSystem,
) -> Result<(), String> {
    let sidecar = resolve_xmp_target(target, filesystem)?;
    editor.edit(&sidecar)
}

#[cfg(test)]
mod tests {
    use crate::usecase::test_helpers::{TestEditor, TestFileSystem};
    use std::path::Path;

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
