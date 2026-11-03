use std::path::PathBuf;

pub trait GlobExpander {
    fn expand(&self, pattern: &str) -> Vec<PathBuf>;
}
