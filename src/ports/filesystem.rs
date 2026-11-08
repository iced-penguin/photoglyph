use std::io;
use std::path::{Path, PathBuf};

/// Provides filesystem operations required by use cases.
pub trait FileSystem {
    /// Resolves a path to its canonical, absolute form.
    fn canonicalize(&self, path: &Path) -> io::Result<PathBuf>;

    /// Returns whether `path` refers to a file.
    fn is_file(&self, path: &Path) -> bool;

    /// Returns whether a filesystem entry exists at `path`.
    fn exists(&self, path: &Path) -> bool;

    /// Lists the paths of entries directly inside a directory.
    fn read_dir(&self, path: &Path) -> io::Result<Vec<PathBuf>>;

    /// Removes a file.
    fn remove_file(&self, path: &Path) -> io::Result<()>;

    /// Installs a temporary sidecar at its destination, optionally replacing it.
    fn install_sidecar(&self, temporary: &Path, sidecar: &Path, replace: bool) -> io::Result<()>;
}
