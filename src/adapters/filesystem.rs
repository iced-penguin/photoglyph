use crate::ports::FileSystem;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};

/// Accesses the local filesystem for application use cases.
#[derive(Default)]
pub struct LocalFileSystem;

impl FileSystem for LocalFileSystem {
    fn canonicalize(&self, path: &Path) -> io::Result<PathBuf> {
        fs::canonicalize(path)
    }

    fn is_file(&self, path: &Path) -> bool {
        path.is_file()
    }

    fn exists(&self, path: &Path) -> bool {
        path.exists()
    }

    fn remove_file(&self, path: &Path) -> io::Result<()> {
        fs::remove_file(path)
    }

    fn install_sidecar(&self, temporary: &Path, sidecar: &Path, replace: bool) -> io::Result<()> {
        if replace {
            fs::rename(temporary, sidecar)
        } else {
            fs::hard_link(temporary, sidecar).and_then(|()| fs::remove_file(temporary))
        }
    }
}
