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

    fn read_dir(&self, path: &Path) -> io::Result<Vec<PathBuf>> {
        collect_entry_paths(fs::read_dir(path)?.map(|entry| entry.map(|entry| entry.path())))
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

fn collect_entry_paths(
    entries: impl IntoIterator<Item = io::Result<PathBuf>>,
) -> io::Result<Vec<PathBuf>> {
    entries.into_iter().collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn collect_entry_paths_returns_all_paths() {
        let entries = vec![Ok(PathBuf::from("/a.jpg")), Ok(PathBuf::from("/b.xmp"))];
        assert_eq!(
            collect_entry_paths(entries).unwrap(),
            vec![PathBuf::from("/a.jpg"), PathBuf::from("/b.xmp")]
        );
    }

    #[test]
    fn collect_entry_paths_propagates_entry_errors() {
        let entries = vec![
            Ok(PathBuf::from("/a.jpg")),
            Err(io::Error::new(io::ErrorKind::PermissionDenied, "denied")),
        ];
        let error = collect_entry_paths(entries).unwrap_err();
        assert_eq!(error.kind(), io::ErrorKind::PermissionDenied);
    }
}
