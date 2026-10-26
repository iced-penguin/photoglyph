use std::io;
use std::path::{Path, PathBuf};

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

pub trait FileSystem {
    fn canonicalize(&self, path: &Path) -> io::Result<PathBuf>;
    fn is_file(&self, path: &Path) -> bool;
    fn exists(&self, path: &Path) -> bool;
    fn read_dir(&self, path: &Path) -> io::Result<Vec<PathBuf>>;
    fn remove_file(&self, path: &Path) -> io::Result<()>;
    fn install_sidecar(&self, temporary: &Path, sidecar: &Path, replace: bool) -> io::Result<()>;
}
