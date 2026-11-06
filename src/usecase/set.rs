use crate::ports::{FileSystem, SidecarStore};
use std::path::Path;

use super::shared::{resolve_xmp_target, validate_key};

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
