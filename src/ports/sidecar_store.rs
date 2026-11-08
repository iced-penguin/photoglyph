use std::path::Path;

/// Which properties to read from a sidecar.
pub enum Selection<'a> {
    /// Read all available XMP properties.
    All,
    /// Read only the listed XMP keys.
    Keys(&'a [&'a str]),
}

/// Reads and writes XMP sidecars; implemented by an external metadata tool.
pub trait SidecarStore {
    /// Checks that the metadata tool is available for use.
    fn ensure_available(&self) -> Result<(), String>;

    /// Exports metadata from an image into a sidecar path.
    fn export_to(&self, image: &Path, sidecar: &Path) -> Result<(), String>;

    /// Reads selected properties and returns their formatted representation.
    fn read_properties(
        &self,
        sidecar: &Path,
        selection: Selection,
        verbose: bool,
    ) -> Result<String, String>;

    /// Sets a property, returning whether the sidecar was changed.
    fn write_property(&self, sidecar: &Path, key: &str, value: &str) -> Result<bool, String>;

    /// Clears a property, returning whether the sidecar was changed.
    fn clear_property(&self, sidecar: &Path, key: &str) -> Result<bool, String>;
}
