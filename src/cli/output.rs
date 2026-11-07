use crate::ports::Logger;
use std::fmt::Display;
use std::io::Write;
use std::path::Path;

/// Writes CLI messages to separate stdout and stderr destinations.
pub struct Output<WOut, WErr> {
    stdout: WOut,
    stderr: WErr,
    failed: bool,
    show_labeled: bool,
    show_printed: bool,
}

impl<WOut: Write, WErr: Write> Output<WOut, WErr> {
    /// Creates an output handler with the provided writers.
    pub fn new(stdout: WOut, stderr: WErr) -> Self {
        Self {
            stdout,
            stderr,
            failed: false,
            show_labeled: false,
            show_printed: false,
        }
    }

    /// Configures show output based on the number of expanded targets.
    pub fn begin_show(&mut self, target_count: usize) {
        self.show_labeled = target_count > 1;
        self.show_printed = false;
    }

    /// Writes a show result, adding a target header and separators when needed.
    pub fn show(&mut self, target: &Path, text: &str) {
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

    /// Writes a success message to stdout or reports an error to stderr.
    pub fn report<T: Display>(&mut self, target: impl Display, result: Result<T, String>) {
        match result {
            Ok(message) => {
                writeln!(self.stdout, "{target}: {message}").expect("failed printing to stdout")
            }
            Err(error) => self.error(target, error),
        }
    }

    /// Reports an error to stderr and ignores successful results.
    pub fn report_silently<T>(&mut self, target: impl Display, result: Result<T, String>) {
        if let Err(error) = result {
            self.error(target, error);
        }
    }

    /// Writes a target-specific error message to stderr.
    pub fn error(&mut self, target: impl Display, error: impl Display) {
        writeln!(self.stderr, "{target}: error: {error}").expect("failed printing to stderr");
        self.failed = true;
    }

    /// Writes a fatal error message to stderr.
    pub fn fatal(&mut self, message: impl Display) {
        writeln!(self.stderr, "{message}").expect("failed printing to stderr");
    }

    /// Writes a clap error to its selected stream and returns whether it is fatal.
    pub fn clap_error(&mut self, error: clap::Error) -> bool {
        let use_stderr = error.use_stderr();
        if use_stderr {
            write!(self.stderr, "{error}").expect("failed printing to stderr");
        } else {
            write!(self.stdout, "{error}").expect("failed printing to stdout");
        }
        use_stderr
    }

    /// Returns whether any target-specific error has been reported.
    pub fn failed(&self) -> bool {
        self.failed
    }

    #[cfg(test)]
    fn into_inner(self) -> (WOut, WErr) {
        (self.stdout, self.stderr)
    }
}

impl<WOut: Write, WErr: Write> Logger for Output<WOut, WErr> {
    fn warn(&mut self, message: &str) {
        writeln!(self.stderr, "warning: {message}").expect("failed printing to stderr");
    }
}

#[cfg(test)]
mod tests {
    use super::Output;
    use clap::{CommandFactory, Parser};
    use std::path::Path;

    #[derive(Parser)]
    #[command(name = "pg")]
    struct Cli {
        #[arg(required = true)]
        target: String,
    }

    #[test]
    fn writes_show_headers_and_separates_multiple_results() {
        let mut output = Output::new(Vec::new(), Vec::new());
        output.begin_show(2);

        output.show(Path::new("one.xmp"), "first");
        output.show(Path::new("two.xmp"), "second");

        let (stdout, stderr) = output.into_inner();
        assert_eq!(stdout, b"== one.xmp ==\nfirst\n\n== two.xmp ==\nsecond\n");
        assert!(stderr.is_empty());
    }

    #[test]
    fn writes_unlabeled_show_output() {
        let mut output = Output::new(Vec::new(), Vec::new());
        output.begin_show(1);

        output.show(Path::new("one.xmp"), "properties");

        let (stdout, stderr) = output.into_inner();
        assert_eq!(stdout, b"properties\n");
        assert!(stderr.is_empty());
    }

    #[test]
    fn writes_headers_and_separators_for_empty_show_results() {
        let mut output = Output::new(Vec::new(), Vec::new());
        output.begin_show(2);

        output.show(Path::new("one.xmp"), "");
        output.show(Path::new("two.xmp"), "");

        let (stdout, stderr) = output.into_inner();
        assert_eq!(stdout, b"== one.xmp ==\n\n== two.xmp ==\n");
        assert!(stderr.is_empty());
    }

    #[test]
    fn leaves_single_empty_show_result_blank() {
        let mut output = Output::new(Vec::new(), Vec::new());
        output.begin_show(1);

        output.show(Path::new("one.xmp"), "");

        let (stdout, stderr) = output.into_inner();
        assert!(stdout.is_empty());
        assert!(stderr.is_empty());
    }

    #[test]
    fn reports_success_and_errors_and_tracks_failure() {
        let mut output = Output::new(Vec::new(), Vec::new());
        output.report("one.jpg", Ok("exported".to_string()));
        output.report("two.jpg", Err::<String, _>("export failed".to_string()));
        output.report_silently("two.xmp", Ok(()));
        output.report_silently("three.xmp", Err::<(), _>("update failed".to_string()));

        assert!(output.failed());
        let (stdout, stderr) = output.into_inner();
        assert_eq!(stdout, b"one.jpg: exported\n");
        assert_eq!(
            stderr,
            b"two.jpg: error: export failed\nthree.xmp: error: update failed\n"
        );
    }

    #[test]
    fn reports_fatal_errors() {
        let mut output = Output::new(Vec::new(), Vec::new());
        output.fatal("pg: exiftool is unavailable");

        let (stdout, stderr) = output.into_inner();
        assert!(stdout.is_empty());
        assert_eq!(stderr, b"pg: exiftool is unavailable\n");
    }

    #[test]
    fn writes_warnings_to_stderr() {
        use crate::ports::Logger;

        let mut output = Output::new(Vec::new(), Vec::new());
        output.warn("ambiguous basename");

        let (stdout, stderr) = output.into_inner();
        assert!(stdout.is_empty());
        assert_eq!(stderr, b"warning: ambiguous basename\n");
    }

    #[test]
    fn routes_clap_errors_to_their_selected_stream() {
        let parse_error = Cli::command().try_get_matches_from(["pg"]).unwrap_err();
        let mut output = Output::new(Vec::new(), Vec::new());
        assert!(output.clap_error(parse_error));
        assert!(!output.failed());
        let (stdout, stderr) = output.into_inner();
        assert!(stdout.is_empty());
        assert!(String::from_utf8(stderr).unwrap().contains("required"));

        let help = Cli::command()
            .try_get_matches_from(["pg", "--help"])
            .unwrap_err();
        let mut output = Output::new(Vec::new(), Vec::new());
        assert!(!output.clap_error(help));
        let (stdout, stderr) = output.into_inner();
        assert!(String::from_utf8(stdout).unwrap().contains("Usage:"));
        assert!(stderr.is_empty());
    }
}
