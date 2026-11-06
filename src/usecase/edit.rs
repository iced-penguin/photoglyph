use crate::ports::{Editor, FileSystem};
use std::path::Path;

use super::shared::resolve_xmp_target;

pub fn edit(
    target: &Path,
    editor: &impl Editor,
    filesystem: &impl FileSystem,
) -> Result<(), String> {
    let sidecar = resolve_xmp_target(target, filesystem)?;
    editor.edit(&sidecar)
}
