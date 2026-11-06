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
