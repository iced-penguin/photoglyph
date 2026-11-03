use crate::ports::GlobExpander;
use glob::{MatchOptions, glob_with};
use std::path::{Path, PathBuf};

#[derive(Default)]
pub struct ShellGlob;

impl GlobExpander for ShellGlob {
    fn expand(&self, pattern: &str) -> Vec<PathBuf> {
        expand(pattern)
    }
}

/// Expands a shell-style pattern; a pattern without metacharacters is returned as is.
pub fn expand(pattern: &str) -> Vec<PathBuf> {
    if !pattern.contains(['*', '?', '[']) || Path::new(pattern).is_file() {
        return vec![PathBuf::from(pattern)];
    }
    let options = MatchOptions {
        case_sensitive: false,
        require_literal_separator: true,
        require_literal_leading_dot: true,
    };
    match glob_with(pattern, options) {
        Ok(paths) => paths.flatten().filter(|p| p.is_file()).collect(),
        // Malformed patterns (e.g. an unclosed '[') are treated as literal paths.
        Err(_) => vec![PathBuf::from(pattern)],
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rstest::rstest;

    #[rstest]
    #[case::literal("photo.jpg")]
    // Invalid glob syntax is preserved as a literal path.
    #[case::malformed("photos/[")]
    fn returns_literal_or_malformed_patterns_unchanged(#[case] pattern: &str) {
        assert_eq!(expand(pattern), vec![PathBuf::from(pattern)]);
    }
}
