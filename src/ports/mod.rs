use std::path::Path;

mod filesystem;
mod glob_expander;
mod sidecar_store;

pub use filesystem::FileSystem;
pub use glob_expander::GlobExpander;
pub use sidecar_store::{Selection, SidecarStore};

/// Opens a sidecar in the user's editor and waits for it to exit.
pub trait Editor {
    fn edit(&self, sidecar: &Path) -> Result<(), String>;
}
