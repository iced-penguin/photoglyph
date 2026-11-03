use std::io;
use std::path::{Path, PathBuf};

pub trait FileSystem {
    fn canonicalize(&self, path: &Path) -> io::Result<PathBuf>;
    fn is_file(&self, path: &Path) -> bool;
    fn exists(&self, path: &Path) -> bool;
    fn read_dir(&self, path: &Path) -> io::Result<Vec<PathBuf>>;
    fn remove_file(&self, path: &Path) -> io::Result<()>;
    fn install_sidecar(&self, temporary: &Path, sidecar: &Path, replace: bool) -> io::Result<()>;
}
