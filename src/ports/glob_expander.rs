use std::path::PathBuf;

/// Expands target patterns for use-case operations.
pub trait GlobExpander {
    /// Returns the paths matched by `pattern`.
    fn expand(&self, pattern: &str) -> Vec<PathBuf>;
}
