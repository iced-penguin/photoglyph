/// Reports non-fatal diagnostics from use cases.
pub trait Logger {
    /// Emits a warning that does not prevent the operation from continuing.
    fn warn(&mut self, message: &str);
}
