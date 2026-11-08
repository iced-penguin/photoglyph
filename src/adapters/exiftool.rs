use crate::ports::{Selection, SidecarStore};
use std::ffi::OsString;
use std::io::{self, ErrorKind};
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

/// Implements sidecar metadata operations by invoking ExifTool.
///
/// The default executable name is `exiftool`, resolved through the process
/// search path.
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

/// What a finished ExifTool process reported.
struct Outcome {
    success: bool,
    status: String,
    stdout: Vec<u8>,
    stderr: Vec<u8>,
}

impl From<Output> for Outcome {
    fn from(out: Output) -> Self {
        Self {
            success: out.status.success(),
            status: out.status.to_string(),
            stdout: out.stdout,
            stderr: out.stderr,
        }
    }
}

impl SidecarStore for ExifTool {
    fn ensure_available(&self) -> Result<(), String> {
        availability(execute(&self.executable, &["-ver".into()]))
    }

    fn export_to(&self, image: &Path, sidecar: &Path) -> Result<(), String> {
        let out = run(&self.executable, &export_args(image, sidecar))?;
        checked_status(&out)
    }

    fn read_properties(
        &self,
        sidecar: &Path,
        selection: Selection,
        verbose: bool,
    ) -> Result<String, String> {
        let out = run(&self.executable, &read_args(sidecar, &selection, verbose))?;
        read_result(&selection, verbose, out)
    }

    fn write_property(&self, sidecar: &Path, key: &str, value: &str) -> Result<bool, String> {
        mutation_result(run(&self.executable, &write_args(sidecar, key, value))?)
    }

    fn clear_property(&self, sidecar: &Path, key: &str) -> Result<bool, String> {
        mutation_result(run(&self.executable, &clear_args(sidecar, key))?)
    }
}

// The only place that spawns a process; everything else is pure and unit-tested.
fn execute(executable: &Path, args: &[OsString]) -> io::Result<Outcome> {
    Command::new(executable)
        .args(args)
        .output()
        .map(Outcome::from)
}

fn run(executable: &Path, args: &[OsString]) -> Result<Outcome, String> {
    execute(executable, args).map_err(|error| format!("cannot run exiftool: {error}"))
}

fn availability(result: io::Result<Outcome>) -> Result<(), String> {
    match result {
        Ok(out) => checked_status(&out),
        Err(error) if error.kind() == ErrorKind::NotFound => Err(INSTALL_HINT.into()),
        Err(error) => Err(format!("cannot run exiftool: {error}")),
    }
}

fn base_args() -> Vec<OsString> {
    vec!["-charset".into(), "utf8".into()]
}

fn export_args(image: &Path, sidecar: &Path) -> Vec<OsString> {
    let mut args = base_args();
    args.extend([
        "-tagsFromFile".into(),
        image.into(),
        "-o".into(),
        sidecar.into(),
        image.into(),
    ]);
    args
}

fn read_args(sidecar: &Path, selection: &Selection, verbose: bool) -> Vec<OsString> {
    let mut args = base_args();
    match (selection, verbose) {
        (Selection::All, true) => args.extend(["-G1".into(), "-a".into(), "-XMP:All".into()]),
        (Selection::All, false) => {
            args.extend(["-args".into(), "-G1".into(), "-a".into(), "-XMP:All".into()])
        }
        (Selection::Keys(keys), true) => {
            args.extend(["-G1".into(), "-a".into()]);
            args.extend(keys.iter().map(|key| OsString::from(format!("-{key}"))));
        }
        (Selection::Keys(keys), false) => {
            for key in *keys {
                args.extend(["-p".into(), format!("{key}: ${{{key}}}").into()]);
            }
        }
    }
    args.push(sidecar.into());
    args
}

fn write_args(sidecar: &Path, key: &str, value: &str) -> Vec<OsString> {
    mutation_args(sidecar, format!("-{key}={value}"))
}

fn clear_args(sidecar: &Path, key: &str) -> Vec<OsString> {
    mutation_args(sidecar, format!("-{key}^="))
}

fn mutation_args(sidecar: &Path, assignment: String) -> Vec<OsString> {
    let mut args = base_args();
    args.extend([
        "-overwrite_original".into(),
        assignment.into(),
        sidecar.into(),
    ]);
    args
}

fn read_result(selection: &Selection, verbose: bool, out: Outcome) -> Result<String, String> {
    let text = checked_text(out)?;
    Ok(match (selection, verbose) {
        (Selection::All, false) => format_all_properties(&text),
        _ => text,
    })
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

fn checked_text(out: Outcome) -> Result<String, String> {
    checked_status(&out)?;
    Ok(String::from_utf8_lossy(&out.stdout).trim_end().to_string())
}

fn checked_status(out: &Outcome) -> Result<(), String> {
    if out.success {
        Ok(())
    } else {
        Err(output_error(out))
    }
}

fn mutation_result(out: Outcome) -> Result<bool, String> {
    checked_status(&out)?;
    let stdout = String::from_utf8_lossy(&out.stdout);
    Ok(stdout.contains("1 image files updated"))
}

fn output_error(out: &Outcome) -> String {
    let detail = String::from_utf8_lossy(&out.stderr).trim().to_string();
    if detail.is_empty() {
        format!("exiftool failed ({})", out.status)
    } else {
        detail
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rstest::rstest;

    fn sidecar() -> &'static Path {
        Path::new("/photos/photo.xmp")
    }

    fn strings(args: Vec<OsString>) -> Vec<String> {
        args.into_iter()
            .map(|arg| arg.to_string_lossy().into_owned())
            .collect()
    }

    fn outcome(success: bool, stdout: &str, stderr: &str) -> Outcome {
        Outcome {
            success,
            status: "exit status: 4".to_string(),
            stdout: stdout.as_bytes().to_vec(),
            stderr: stderr.as_bytes().to_vec(),
        }
    }

    fn ok(stdout: &str) -> Outcome {
        outcome(true, stdout, "")
    }

    fn failed(stderr: &str) -> Outcome {
        outcome(false, "", stderr)
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
    fn defaults_to_exiftool_on_the_path() {
        assert_eq!(ExifTool::default().executable, PathBuf::from("exiftool"));
    }

    #[test]
    fn builds_export_arguments() {
        assert_eq!(
            strings(export_args(
                Path::new("/photos/photo.jpg"),
                Path::new("/photos/photo.xmp")
            )),
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
    #[case::all_verbose(
        Selection::All,
        true,
        &["-charset", "utf8", "-G1", "-a", "-XMP:All", "/photos/photo.xmp"]
    )]
    #[case::all_concise(
        Selection::All,
        false,
        &["-charset", "utf8", "-args", "-G1", "-a", "-XMP:All", "/photos/photo.xmp"]
    )]
    #[case::keys_verbose(
        Selection::Keys(&["XMP-dc:Description", "XMP-dc:Creator"]),
        true,
        &["-charset", "utf8", "-G1", "-a", "-XMP-dc:Description", "-XMP-dc:Creator", "/photos/photo.xmp"]
    )]
    #[case::keys_concise(
        Selection::Keys(&["XMP-dc:Description", "XMP-dc:Creator"]),
        false,
        &[
            "-charset",
            "utf8",
            "-p",
            "XMP-dc:Description: ${XMP-dc:Description}",
            "-p",
            "XMP-dc:Creator: ${XMP-dc:Creator}",
            "/photos/photo.xmp",
        ]
    )]
    fn builds_read_arguments(
        #[case] selection: Selection,
        #[case] verbose: bool,
        #[case] expected: &[&str],
    ) {
        assert_eq!(strings(read_args(sidecar(), &selection, verbose)), expected);
    }

    #[rstest]
    #[case::write(
        write_args(sidecar(), "XMP-dc:Description", "hello"),
        "-XMP-dc:Description=hello"
    )]
    #[case::clear(clear_args(sidecar(), "XMP-dc:Description"), "-XMP-dc:Description^=")]
    fn builds_mutation_arguments(#[case] args: Vec<OsString>, #[case] assignment: &str) {
        assert_eq!(
            strings(args),
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
    #[case::all_concise(
        Selection::All,
        false,
        "-XMP-dc:Description=hello\n",
        "XMP-dc:Description: hello"
    )]
    #[case::all_verbose(
        Selection::All,
        true,
        "XMP-dc:Description : hello\n",
        "XMP-dc:Description : hello"
    )]
    #[case::keys_concise(Selection::Keys(&["a"]), false, "a: 1=2\n", "a: 1=2")]
    fn interprets_read_output(
        #[case] selection: Selection,
        #[case] verbose: bool,
        #[case] stdout: &str,
        #[case] expected: &str,
    ) {
        assert_eq!(
            read_result(&selection, verbose, ok(stdout)).unwrap(),
            expected
        );
    }

    #[test]
    fn propagates_stderr_when_the_process_fails() {
        assert_eq!(
            read_result(&Selection::All, false, failed("command failed\n")).unwrap_err(),
            "command failed"
        );
        assert_eq!(
            checked_status(&failed("command failed")).unwrap_err(),
            "command failed"
        );
        assert_eq!(
            mutation_result(failed("command failed")).unwrap_err(),
            "command failed"
        );
        assert_eq!(
            availability(Ok(failed("command failed"))).unwrap_err(),
            "command failed"
        );
    }

    #[test]
    fn falls_back_to_status_when_failed_process_has_no_stderr() {
        assert_eq!(
            checked_status(&failed("")).unwrap_err(),
            "exiftool failed (exit status: 4)"
        );
    }

    #[rstest]
    #[case::updated("1 image files updated\n", true)]
    #[case::unchanged("0 image files updated\n", false)]
    fn reports_write_and_clear_update_status(#[case] stdout: &str, #[case] expected: bool) {
        assert_eq!(mutation_result(ok(stdout)).unwrap(), expected);
    }

    #[test]
    fn reports_availability() {
        assert!(availability(Ok(ok("12.00\n"))).is_ok());
        assert_eq!(
            availability(Err(io::Error::from(ErrorKind::NotFound))).unwrap_err(),
            INSTALL_HINT
        );
        assert!(
            availability(Err(io::Error::from(ErrorKind::PermissionDenied)))
                .unwrap_err()
                .starts_with("cannot run exiftool: ")
        );
    }
}
