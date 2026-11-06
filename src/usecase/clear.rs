use crate::ports::{FileSystem, SidecarStore};
use std::path::Path;

use super::shared::{resolve_xmp_target, validate_key};

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
