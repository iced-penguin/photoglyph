use crate::port::FileSystem;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};

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

    fn read_dir(&self, path: &Path) -> io::Result<Vec<PathBuf>> {
        Ok(fs::read_dir(path)?
            .flatten()
            .map(|entry| entry.path())
            .collect())
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
