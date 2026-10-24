use glob::{MatchOptions, glob_with};
use std::path::{Path, PathBuf};

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
    use std::fs;

    #[test]
    fn expands_recursively() {
        let dir = std::env::temp_dir().join(format!("pg-glob-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(dir.join("a/b")).unwrap();
        fs::create_dir_all(dir.join(".hidden")).unwrap();
        for f in [
            "x.jpg",
            "a/y.JPG",
            "a/b/z.jpg",
            "a/b/z.txt",
            ".hidden/h.jpg",
        ] {
            fs::write(dir.join(f), b"").unwrap();
        }
        let d = glob::Pattern::escape(&dir.to_string_lossy());
        assert_eq!(expand(&format!("{d}/*.jpg")).len(), 1);
        assert_eq!(expand(&format!("{d}/**/*.jpg")).len(), 3);
        assert_eq!(expand(&format!("{d}/a/*/z.*")).len(), 2);
        assert_eq!(expand(&format!("{d}/nothing")).len(), 1);
        assert_eq!(expand(&format!("{d}/a/[")).len(), 1);
        fs::remove_dir_all(&dir).unwrap();
    }
}
