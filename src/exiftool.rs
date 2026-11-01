use crate::port::{Selection, SidecarStore};
use std::io::ErrorKind;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

pub struct ExifTool {
    executable: PathBuf,
}

impl Default for ExifTool {
    fn default() -> Self {
        Self {
            executable: PathBuf::from("exiftool"),
        }
    }
}

const INSTALL_HINT: &str = "exiftool not found. Install it and try again:
  macOS:           brew install exiftool
  Debian/Ubuntu:   sudo apt install libimage-exiftool-perl
  other:           https://exiftool.org";

impl SidecarStore for ExifTool {
    fn ensure_available(&self) -> Result<(), String> {
        match Command::new(&self.executable).arg("-ver").output() {
            Ok(out) if out.status.success() => Ok(()),
            Ok(out) => Err(output_error(&out)),
            Err(error) if error.kind() == ErrorKind::NotFound => Err(INSTALL_HINT.into()),
            Err(error) => Err(format!("cannot run exiftool: {error}")),
        }
    }

    fn export_to(&self, image: &Path, sidecar: &Path) -> Result<(), String> {
        let out = Command::new(&self.executable)
            .args(["-charset", "utf8", "-tagsFromFile"])
            .arg(image)
            .arg("-o")
            .arg(sidecar)
            .arg(image)
            .output()
            .map_err(|error| format!("cannot run exiftool: {error}"))?;
        checked_status(&out)
    }

    fn read_properties(
        &self,
        sidecar: &Path,
        selection: Selection,
        verbose: bool,
    ) -> Result<String, String> {
        match selection {
            Selection::All => read_all(&self.executable, sidecar, verbose),
            Selection::Keys(keys) => read_keys(&self.executable, sidecar, keys, verbose),
        }
    }

    fn write_property(&self, sidecar: &Path, key: &str, value: &str) -> Result<bool, String> {
        mutation_result(run(
            &self.executable,
            &["-overwrite_original", &format!("-{key}={value}")],
            sidecar,
        )?)
    }

    fn clear_property(&self, sidecar: &Path, key: &str) -> Result<bool, String> {
        mutation_result(run(
            &self.executable,
            &["-overwrite_original", &format!("-{key}^=")],
            sidecar,
        )?)
    }
}

fn read_keys(
    executable: &Path,
    sidecar: &Path,
    keys: &[&str],
    verbose: bool,
) -> Result<String, String> {
    if verbose {
        let selectors: Vec<String> = keys.iter().map(|key| format!("-{key}")).collect();
        let mut args = vec!["-G1", "-a"];
        args.extend(selectors.iter().map(String::as_str));
        return checked_text(run(executable, &args, sidecar)?);
    }

    let formats: Vec<String> = keys
        .iter()
        .map(|key| format!("{key}: ${{{key}}}"))
        .collect();
    let mut args = Vec::new();
    for format in &formats {
        args.extend(["-p", format]);
    }
    checked_text(run(executable, &args, sidecar)?)
}

fn read_all(executable: &Path, sidecar: &Path, verbose: bool) -> Result<String, String> {
    if verbose {
        return checked_text(run(executable, &["-G1", "-a", "-XMP:All"], sidecar)?);
    }
    let text = checked_text(run(
        executable,
        &["-args", "-G1", "-a", "-XMP:All"],
        sidecar,
    )?)?;
    Ok(format_all_properties(&text))
}

fn format_all_properties(text: &str) -> String {
    text.lines()
        .map(|line| {
            let line = line.strip_prefix('-').unwrap_or(line);
            match line.split_once('=') {
                Some((key, value)) => format!("{key}: {value}"),
                None => line.to_string(),
            }
        })
        .collect::<Vec<_>>()
        .join("\n")
}

fn run(executable: &Path, args: &[&str], file: &Path) -> Result<Output, String> {
    Command::new(executable)
        .args(["-charset", "utf8"])
        .args(args)
        .arg(file)
        .output()
        .map_err(|error| format!("cannot run exiftool: {error}"))
}

fn checked_text(out: Output) -> Result<String, String> {
    checked_status(&out)?;
    Ok(String::from_utf8_lossy(&out.stdout).trim_end().to_string())
}

fn checked_status(out: &Output) -> Result<(), String> {
    if out.status.success() {
        Ok(())
    } else {
        Err(output_error(out))
    }
}

fn mutation_result(out: Output) -> Result<bool, String> {
    checked_status(&out)?;
    let stdout = String::from_utf8_lossy(&out.stdout);
    Ok(stdout.contains("1 image files updated"))
}

fn output_error(out: &Output) -> String {
    let detail = String::from_utf8_lossy(&out.stderr).trim().to_string();
    if detail.is_empty() {
        format!("exiftool failed ({})", out.status)
    } else {
        detail
    }
}

#[cfg(test)]
mod tests {
    use super::{ExifTool, format_all_properties};
    use crate::port::{Selection, SidecarStore};
    use rstest::rstest;
    use std::fs;
    use std::os::unix::fs::PermissionsExt;
    use std::path::{Path, PathBuf};
    use std::sync::atomic::{AtomicUsize, Ordering};

    struct TestTool {
        directory: PathBuf,
        tool: ExifTool,
    }

    #[derive(Clone, Copy)]
    enum Operation {
        EnsureAvailable,
        Export,
        ReadKeys,
        ReadAll,
        Write,
        Clear,
    }

    impl Operation {
        fn run(self, tool: &ExifTool) -> Result<(), String> {
            match self {
                Self::EnsureAvailable => tool.ensure_available(),
                Self::Export => tool.export_to(Path::new("/photos/photo.jpg"), sidecar()),
                Self::ReadKeys => tool
                    .read_properties(sidecar(), Selection::Keys(&["XMP-dc:Description"]), false)
                    .map(|_| ()),
                Self::ReadAll => tool
                    .read_properties(sidecar(), Selection::All, false)
                    .map(|_| ()),
                Self::Write => tool
                    .write_property(sidecar(), "XMP-dc:Description", "hello")
                    .map(|_| ()),
                Self::Clear => tool
                    .clear_property(sidecar(), "XMP-dc:Description")
                    .map(|_| ()),
            }
        }

        fn missing_executable_error(self) -> &'static str {
            match self {
                Self::EnsureAvailable => "exiftool not found",
                Self::Export | Self::ReadKeys | Self::ReadAll | Self::Write | Self::Clear => {
                    "cannot run exiftool"
                }
            }
        }
    }

    impl TestTool {
        fn new(stdout: &[u8], stderr: &[u8], exit_code: i32) -> Self {
            static NEXT_ID: AtomicUsize = AtomicUsize::new(0);
            let directory = std::env::temp_dir().join(format!(
                "pg-exiftool-{}-{}",
                std::process::id(),
                NEXT_ID.fetch_add(1, Ordering::Relaxed)
            ));
            fs::create_dir(&directory).unwrap();
            let executable = directory.join("exiftool");
            let script = format!(
                "#!/bin/sh\nprintf '%s' '{}'\nprintf '%s' '{}' >&2\nexit {}\n",
                shell_quote(stdout),
                shell_quote(stderr),
                exit_code
            );
            fs::write(&executable, script).unwrap();
            let mut permissions = fs::metadata(&executable).unwrap().permissions();
            permissions.set_mode(0o755);
            fs::set_permissions(&executable, permissions).unwrap();

            Self {
                directory,
                tool: ExifTool { executable },
            }
        }

        fn echo_args() -> Self {
            static NEXT_ID: AtomicUsize = AtomicUsize::new(0);
            let directory = std::env::temp_dir().join(format!(
                "pg-exiftool-args-{}-{}",
                std::process::id(),
                NEXT_ID.fetch_add(1, Ordering::Relaxed)
            ));
            fs::create_dir(&directory).unwrap();
            let executable = directory.join("exiftool");
            fs::write(&executable, "#!/bin/sh\nprintf '%s\\n' \"$@\"\n").unwrap();
            let mut permissions = fs::metadata(&executable).unwrap().permissions();
            permissions.set_mode(0o755);
            fs::set_permissions(&executable, permissions).unwrap();
            Self {
                directory,
                tool: ExifTool { executable },
            }
        }
    }

    impl TestTool {
        fn recording_args() -> Self {
            static NEXT_ID: AtomicUsize = AtomicUsize::new(0);
            let directory = std::env::temp_dir().join(format!(
                "pg-exiftool-record-{}-{}",
                std::process::id(),
                NEXT_ID.fetch_add(1, Ordering::Relaxed)
            ));
            fs::create_dir(&directory).unwrap();
            let executable = directory.join("exiftool");
            fs::write(
                &executable,
                "#!/bin/sh\nprintf '%s\\n' \"$@\" > \"$0.args\"\n",
            )
            .unwrap();
            let mut permissions = fs::metadata(&executable).unwrap().permissions();
            permissions.set_mode(0o755);
            fs::set_permissions(&executable, permissions).unwrap();
            Self {
                directory,
                tool: ExifTool { executable },
            }
        }

        fn recorded_args(&self) -> Vec<String> {
            fs::read_to_string(self.directory.join("exiftool.args"))
                .unwrap()
                .lines()
                .map(str::to_string)
                .collect()
        }
    }

    impl Drop for TestTool {
        fn drop(&mut self) {
            fs::remove_dir_all(&self.directory).unwrap();
        }
    }

    fn shell_quote(bytes: &[u8]) -> String {
        String::from_utf8_lossy(bytes).replace('\'', "'\"'\"'")
    }

    fn sidecar() -> &'static Path {
        Path::new("/photos/photo.xmp")
    }

    #[rstest]
    #[case::formats_properties(
        "-XMP-dc:Description=hello\n-XMP-xmp:Rating=5",
        "XMP-dc:Description: hello\nXMP-xmp:Rating: 5"
    )]
    #[case::preserves_equals_and_unparsed_lines(
        "-XMP-dc:Description=a=b\nunparsed line",
        "XMP-dc:Description: a=b\nunparsed line"
    )]
    #[case::formats_empty_output("", "")]
    fn formats_exiftool_argument_output(#[case] input: &str, #[case] expected: &str) {
        assert_eq!(format_all_properties(input), expected);
    }

    #[test]
    fn checks_exiftool_availability() {
        assert_eq!(ExifTool::default().executable, PathBuf::from("exiftool"));

        let available = TestTool::new(b"12.00\n", b"", 0);
        assert!(available.tool.ensure_available().is_ok());

        let not_executable = TestTool::new(b"", b"", 0);
        let mut permissions = fs::metadata(&not_executable.tool.executable)
            .unwrap()
            .permissions();
        permissions.set_mode(0o644);
        fs::set_permissions(&not_executable.tool.executable, permissions).unwrap();
        assert!(
            not_executable
                .tool
                .ensure_available()
                .unwrap_err()
                .contains("cannot run exiftool")
        );
    }

    #[rstest]
    #[case::ensure_available(Operation::EnsureAvailable)]
    #[case::export(Operation::Export)]
    #[case::read_keys(Operation::ReadKeys)]
    #[case::read_all(Operation::ReadAll)]
    #[case::write(Operation::Write)]
    #[case::clear(Operation::Clear)]
    fn propagates_exiftool_stderr_on_failure(#[case] operation: Operation) {
        let failed = TestTool::new(b"", b"command failed", 1);
        assert_eq!(operation.run(&failed.tool).unwrap_err(), "command failed");
    }

    #[rstest]
    #[case::ensure_available(Operation::EnsureAvailable)]
    #[case::export(Operation::Export)]
    // read/write/clear all spawn the process through `run`, so one of them represents the rest.
    #[case::shared_run(Operation::ReadKeys)]
    fn reports_missing_executable(#[case] operation: Operation) {
        let missing = ExifTool {
            executable: PathBuf::from("/missing/exiftool"),
        };
        assert!(
            operation
                .run(&missing)
                .unwrap_err()
                .contains(operation.missing_executable_error())
        );
    }

    #[test]
    fn exports_with_exiftool() {
        let image = Path::new("/photos/photo.jpg");
        let success = TestTool::recording_args();
        assert!(success.tool.export_to(image, sidecar()).is_ok());
        assert_eq!(
            success.recorded_args(),
            [
                "-charset",
                "utf8",
                "-tagsFromFile",
                "/photos/photo.jpg",
                "-o",
                "/photos/photo.xmp",
                "/photos/photo.jpg",
            ]
        );
    }

    #[rstest]
    #[case::write(Operation::Write, "-XMP-dc:Description=hello")]
    #[case::clear(Operation::Clear, "-XMP-dc:Description^=")]
    fn passes_mutation_arguments_to_exiftool(
        #[case] operation: Operation,
        #[case] assignment: &str,
    ) {
        let tool = TestTool::recording_args();
        operation.run(&tool.tool).unwrap();
        assert_eq!(
            tool.recorded_args(),
            [
                "-charset",
                "utf8",
                "-overwrite_original",
                assignment,
                "/photos/photo.xmp",
            ]
        );
    }

    #[rstest]
    #[case::concise(false)]
    #[case::verbose(true)]
    fn reads_selected_keys_in_concise_and_verbose_formats(#[case] verbose: bool) {
        let tool = TestTool::echo_args();
        let keys = ["XMP-dc:Description", "XMP-dc:Creator"];
        assert_eq!(
            tool
                .tool
                .read_properties(sidecar(), Selection::Keys(&keys), verbose)
                .unwrap(),
            if verbose {
                format!(
                    "-charset\nutf8\n-G1\n-a\n-XMP-dc:Description\n-XMP-dc:Creator\n{}\n",
                    sidecar().display()
                )
            } else {
                format!(
                    "-charset\nutf8\n-p\nXMP-dc:Description: ${{XMP-dc:Description}}\n-p\nXMP-dc:Creator: ${{XMP-dc:Creator}}\n{}\n",
                    sidecar().display()
                )
            }
            .trim_end()
        );
    }

    #[rstest]
    #[case::concise(false)]
    #[case::verbose(true)]
    fn reads_all_properties_in_concise_and_verbose_formats(#[case] verbose: bool) {
        let tool = if verbose {
            TestTool::echo_args()
        } else {
            TestTool::new(b"-XMP-dc:Description=hello\n-XMP-xmp:Rating=5\n", b"", 0)
        };
        assert_eq!(
            tool.tool
                .read_properties(sidecar(), Selection::All, verbose)
                .unwrap(),
            if verbose {
                format!(
                    "-charset\nutf8\n-G1\n-a\n-XMP:All\n{}\n",
                    sidecar().display()
                )
            } else {
                "XMP-dc:Description: hello\nXMP-xmp:Rating: 5".to_string()
            }
            .trim_end()
        );
    }

    #[test]
    fn falls_back_to_status_when_failed_process_has_no_stderr() {
        let failed = TestTool::new(b"", b"", 4);
        assert!(
            failed
                .tool
                .read_properties(sidecar(), Selection::All, false)
                .unwrap_err()
                .contains("exiftool failed")
        );
    }

    #[rstest]
    #[case::updated(b"1 image files updated\n", true)]
    #[case::unchanged(b"0 image files updated\n", false)]
    fn reports_write_and_clear_update_status(
        #[case] stdout: &[u8],
        #[case] expected_updated: bool,
    ) {
        let tool = TestTool::new(stdout, b"", 0);
        assert_eq!(
            tool.tool
                .write_property(sidecar(), "XMP-dc:Description", "hello")
                .unwrap(),
            expected_updated
        );
        assert_eq!(
            tool.tool
                .clear_property(sidecar(), "XMP-dc:Description")
                .unwrap(),
            expected_updated
        );
    }
}
