use crate::port::{Editor, FileSystem, Selection, SidecarStore};
use std::cell::RefCell;
use std::collections::{HashMap, HashSet};
use std::io::{self, ErrorKind};
use std::path::{Path, PathBuf};
use std::rc::Rc;

#[derive(Clone, Default)]
pub(super) struct TestFileSystem(pub(super) Rc<RefCell<FsState>>);

#[derive(Default)]
pub(super) struct FsState {
    pub(super) files: HashSet<PathBuf>,
    pub(super) directories: HashMap<PathBuf, Vec<PathBuf>>,
    pub(super) canonicalize_error: Option<ErrorKind>,
    pub(super) canonicalize_to: Option<PathBuf>,
    pub(super) read_dir_error: Option<ErrorKind>,
    pub(super) install_error: Option<ErrorKind>,
    pub(super) create_sidecar_during_install: bool,
}

impl TestFileSystem {
    pub(super) fn add_file(&self, path: &Path) {
        let mut state = self.0.borrow_mut();
        if state.files.insert(path.to_path_buf())
            && let Some(parent) = path.parent()
        {
            state
                .directories
                .entry(parent.to_path_buf())
                .or_default()
                .push(path.to_path_buf());
        }
    }

    pub(super) fn file_count(&self) -> usize {
        self.0.borrow().files.len()
    }
}

impl FileSystem for TestFileSystem {
    fn canonicalize(&self, path: &Path) -> io::Result<PathBuf> {
        let state = self.0.borrow();
        if let Some(kind) = state.canonicalize_error {
            return Err(io::Error::from(kind));
        }
        if state.files.contains(path) {
            Ok(state
                .canonicalize_to
                .clone()
                .unwrap_or_else(|| path.to_path_buf()))
        } else {
            Err(io::Error::from(ErrorKind::NotFound))
        }
    }

    fn is_file(&self, path: &Path) -> bool {
        self.0.borrow().files.contains(path)
    }

    fn exists(&self, path: &Path) -> bool {
        self.0.borrow().files.contains(path)
    }

    fn read_dir(&self, path: &Path) -> io::Result<Vec<PathBuf>> {
        let state = self.0.borrow();
        if let Some(kind) = state.read_dir_error {
            return Err(io::Error::from(kind));
        }
        state
            .directories
            .get(path)
            .cloned()
            .ok_or_else(|| io::Error::from(ErrorKind::NotFound))
    }

    fn remove_file(&self, path: &Path) -> io::Result<()> {
        let mut state = self.0.borrow_mut();
        if !state.files.remove(path) {
            return Err(io::Error::from(ErrorKind::NotFound));
        }
        if let Some(parent) = path.parent()
            && let Some(entries) = state.directories.get_mut(parent)
        {
            entries.retain(|entry| entry != path);
        }
        Ok(())
    }

    fn install_sidecar(&self, temporary: &Path, sidecar: &Path, replace: bool) -> io::Result<()> {
        let state = self.0.borrow();
        let install_error = state.install_error;
        let create_sidecar_during_install = state.create_sidecar_during_install;
        drop(state);
        if let Some(kind) = install_error {
            if create_sidecar_during_install {
                self.add_file(sidecar);
            }
            return Err(io::Error::from(kind));
        }
        if !replace && self.exists(sidecar) {
            return Err(io::Error::from(ErrorKind::AlreadyExists));
        }
        self.remove_file(temporary)?;
        self.add_file(sidecar);
        Ok(())
    }
}

#[derive(Default)]
pub(super) struct TestSidecarStore {
    pub(super) filesystem: TestFileSystem,
    pub(super) export_error: Option<String>,
    pub(super) export_creates_file: bool,
    pub(super) read_error: Option<String>,
    pub(super) read_text: Option<String>,
    pub(super) write_result: Option<Result<bool, String>>,
    pub(super) clear_result: Option<Result<bool, String>>,
    pub(super) calls: RefCell<Vec<StoreCall>>,
}

#[derive(Debug, PartialEq)]
pub(super) enum StoreCall {
    Export(PathBuf, PathBuf),
    Read(PathBuf, String, bool),
    Write(PathBuf, String, String),
    Clear(PathBuf, String),
}

impl SidecarStore for TestSidecarStore {
    fn ensure_available(&self) -> Result<(), String> {
        Ok(())
    }

    fn export_to(&self, image: &Path, sidecar: &Path) -> Result<(), String> {
        self.calls.borrow_mut().push(StoreCall::Export(
            image.to_path_buf(),
            sidecar.to_path_buf(),
        ));
        if self.export_creates_file {
            self.filesystem.add_file(sidecar);
        }
        self.export_error.clone().map_or(Ok(()), Err)
    }

    fn read_properties(
        &self,
        sidecar: &Path,
        selection: Selection,
        verbose: bool,
    ) -> Result<String, String> {
        let selection = match selection {
            Selection::All => "all".to_string(),
            Selection::Keys(keys) => keys.join(","),
        };
        self.calls
            .borrow_mut()
            .push(StoreCall::Read(sidecar.to_path_buf(), selection, verbose));
        if let Some(error) = &self.read_error {
            return Err(error.clone());
        }
        Ok(self
            .read_text
            .clone()
            .unwrap_or_else(|| "properties".to_string()))
    }

    fn write_property(&self, sidecar: &Path, key: &str, value: &str) -> Result<bool, String> {
        self.calls.borrow_mut().push(StoreCall::Write(
            sidecar.to_path_buf(),
            key.to_string(),
            value.to_string(),
        ));
        self.write_result.clone().unwrap_or(Ok(true))
    }

    fn clear_property(&self, sidecar: &Path, key: &str) -> Result<bool, String> {
        self.calls
            .borrow_mut()
            .push(StoreCall::Clear(sidecar.to_path_buf(), key.to_string()));
        self.clear_result.clone().unwrap_or(Ok(true))
    }
}

#[derive(Default)]
pub(super) struct TestEditor {
    pub(super) error: Option<String>,
    pub(super) opened: RefCell<Vec<PathBuf>>,
}

impl Editor for TestEditor {
    fn open(&self, sidecar: &Path) -> Result<(), String> {
        self.opened.borrow_mut().push(sidecar.to_path_buf());
        self.error.clone().map_or(Ok(()), Err)
    }
}
