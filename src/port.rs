use std::path::Path;

/// Which properties to read from a sidecar.
pub enum Selection<'a> {
    All,
    Keys(&'a [&'a str]),
}

/// Reads and writes XMP sidecars; implemented by an external metadata tool.
pub trait SidecarStore {
    fn ensure_available(&self) -> Result<(), String>;
    fn export_to(&self, image: &Path, sidecar: &Path) -> Result<(), String>;
    fn read_properties(
        &self,
        sidecar: &Path,
        selection: Selection,
        verbose: bool,
    ) -> Result<String, String>;
    fn write_property(&self, sidecar: &Path, key: &str, value: &str) -> Result<bool, String>;
    fn clear_property(&self, sidecar: &Path, key: &str) -> Result<bool, String>;
}
