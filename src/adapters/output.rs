use crate::ports::{Logger, Output};
use std::io::Write;
use std::path::Path;

/// Writes command output to stdout and errors or warnings to stderr.
pub struct CliOutput<WOut, WErr> {
    stdout: WOut,
    stderr: WErr,
    show_labeled: bool,
    show_printed: bool,
}

impl<WOut: Write, WErr: Write> CliOutput<WOut, WErr> {
    /// Creates an output adapter using the provided writers.
    pub fn new(stdout: WOut, stderr: WErr) -> Self {
        Self {
            stdout,
            stderr,
            show_labeled: false,
            show_printed: false,
        }
    }

    #[cfg(test)]
    fn into_inner(self) -> (WOut, WErr) {
        (self.stdout, self.stderr)
    }
}

impl<WOut: Write, WErr: Write> Output for CliOutput<WOut, WErr> {
    fn begin_show(&mut self, target_count: usize) {
        self.show_labeled = target_count > 1;
        self.show_printed = false;
    }

    fn show(&mut self, target: &Path, text: &str) {
        if self.show_printed {
            writeln!(self.stdout).expect("failed printing to stdout");
        }
        if self.show_labeled {
            writeln!(self.stdout, "== {} ==", target.display()).expect("failed printing to stdout");
        }
        if !text.is_empty() {
            writeln!(self.stdout, "{text}").expect("failed printing to stdout");
        }
        self.show_printed = true;
    }

    fn success(&mut self, target: &Path, message: &str) {
        writeln!(self.stdout, "{}: {message}", target.display())
            .expect("failed printing to stdout");
    }

    fn error(&mut self, target: &Path, message: &str) {
        writeln!(self.stderr, "{}: error: {message}", target.display())
            .expect("failed printing to stderr");
    }

    fn fatal(&mut self, message: &str) {
        writeln!(self.stderr, "{message}").expect("failed printing to stderr");
    }
}

impl<WOut: Write, WErr: Write> Logger for CliOutput<WOut, WErr> {
    fn warn(&mut self, message: &str) {
        writeln!(self.stderr, "warning: {message}").expect("failed printing to stderr");
    }
}

#[cfg(test)]
mod tests {
    use super::CliOutput;
    use crate::ports::{Logger, Output};
    use std::path::Path;

    #[test]
    fn writes_show_headers_and_separates_multiple_results() {
        let mut output = CliOutput::new(Vec::new(), Vec::new());
        output.begin_show(2);

        output.show(Path::new("one.xmp"), "first");
        output.show(Path::new("two.xmp"), "second");

        let (stdout, stderr) = output.into_inner();
        assert_eq!(stdout, b"== one.xmp ==\nfirst\n\n== two.xmp ==\nsecond\n");
        assert!(stderr.is_empty());
    }

    #[test]
    fn writes_unlabeled_show_output() {
        let mut output = CliOutput::new(Vec::new(), Vec::new());
        output.begin_show(1);

        output.show(Path::new("one.xmp"), "properties");

        let (stdout, stderr) = output.into_inner();
        assert_eq!(stdout, b"properties\n");
        assert!(stderr.is_empty());
    }

    #[test]
    fn writes_headers_and_separators_for_empty_show_results() {
        let mut output = CliOutput::new(Vec::new(), Vec::new());
        output.begin_show(2);

        output.show(Path::new("one.xmp"), "");
        output.show(Path::new("two.xmp"), "");

        let (stdout, stderr) = output.into_inner();
        assert_eq!(stdout, b"== one.xmp ==\n\n== two.xmp ==\n");
        assert!(stderr.is_empty());
    }

    #[test]
    fn leaves_single_empty_show_result_blank() {
        let mut output = CliOutput::new(Vec::new(), Vec::new());
        output.begin_show(1);

        output.show(Path::new("one.xmp"), "");

        let (stdout, stderr) = output.into_inner();
        assert!(stdout.is_empty());
        assert!(stderr.is_empty());
    }

    #[test]
    fn writes_success_errors_and_fatal_messages_to_the_expected_streams() {
        let mut output = CliOutput::new(Vec::new(), Vec::new());
        output.success(Path::new("one.jpg"), "exported");
        output.error(Path::new("two.xmp"), "update failed");
        output.fatal("pg: exiftool is unavailable");

        let (stdout, stderr) = output.into_inner();
        assert_eq!(stdout, b"one.jpg: exported\n");
        assert_eq!(
            stderr,
            b"two.xmp: error: update failed\npg: exiftool is unavailable\n"
        );
    }

    #[test]
    fn writes_warnings_to_stderr() {
        let mut output = CliOutput::new(Vec::new(), Vec::new());
        output.warn("ambiguous basename");

        let (stdout, stderr) = output.into_inner();
        assert!(stdout.is_empty());
        assert_eq!(stderr, b"warning: ambiguous basename\n");
    }
}
