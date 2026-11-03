use std::path::Path;

/// Opens a sidecar in the user's editor and waits for it to exit.
pub trait Editor {
    fn edit(&self, sidecar: &Path) -> Result<(), String>;
}
