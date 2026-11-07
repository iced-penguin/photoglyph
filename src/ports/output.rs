use super::Logger;
use std::path::Path;

/// Presents command results and diagnostics to the user.
pub trait Output: Logger {
    /// Starts a show operation with the number of expanded targets.
    fn begin_show(&mut self, target_count: usize);

    /// Presents properties read from one target.
    fn show(&mut self, target: &Path, text: &str);

    /// Reports a successful operation.
    fn success(&mut self, target: &Path, message: &str);

    /// Reports a target-specific error.
    fn error(&mut self, target: &Path, message: &str);

    /// Reports a fatal command error.
    fn fatal(&mut self, message: &str);
}
