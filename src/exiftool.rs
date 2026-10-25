use crate::port::{Selection, SidecarStore};
use std::io::ErrorKind;
use std::path::Path;
use std::process::{Command, Output};

#[derive(Default)]
pub struct ExifTool;

const INSTALL_HINT: &str = "exiftool not found. Install it and try again:
  macOS:           brew install exiftool
  Debian/Ubuntu:   sudo apt install libimage-exiftool-perl
  other:           https://exiftool.org";

impl SidecarStore for ExifTool {
    fn ensure_available(&self) -> Result<(), String> {
        match Command::new("exiftool").arg("-ver").output() {
            Ok(out) if out.status.success() => Ok(()),
            Ok(out) => Err(output_error(&out)),
            Err(error) if error.kind() == ErrorKind::NotFound => Err(INSTALL_HINT.into()),
            Err(error) => Err(format!("cannot run exiftool: {error}")),
        }
    }

    fn export_to(&self, image: &Path, sidecar: &Path) -> Result<(), String> {
        let out = Command::new("exiftool")
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
            Selection::All => read_all(sidecar, verbose),
            Selection::Keys(keys) => read_keys(sidecar, keys, verbose),
        }
    }

    fn write_property(&self, sidecar: &Path, key: &str, value: &str) -> Result<bool, String> {
        mutation_result(run(
            &["-overwrite_original", &format!("-{key}={value}")],
            sidecar,
        )?)
    }

    fn clear_property(&self, sidecar: &Path, key: &str) -> Result<bool, String> {
        mutation_result(run(
            &["-overwrite_original", &format!("-{key}^=")],
            sidecar,
        )?)
    }
}

fn read_keys(sidecar: &Path, keys: &[&str], verbose: bool) -> Result<String, String> {
    if verbose {
        let selectors: Vec<String> = keys.iter().map(|key| format!("-{key}")).collect();
        let mut args = vec!["-G1", "-a"];
        args.extend(selectors.iter().map(String::as_str));
        return checked_text(run(&args, sidecar)?);
    }

    let formats: Vec<String> = keys
        .iter()
        .map(|key| format!("{key}: ${{{key}}}"))
        .collect();
    let mut args = Vec::new();
    for format in &formats {
        args.extend(["-p", format]);
    }
    checked_text(run(&args, sidecar)?)
}

fn read_all(sidecar: &Path, verbose: bool) -> Result<String, String> {
    if verbose {
        return checked_text(run(&["-G1", "-a", "-XMP:All"], sidecar)?);
    }
    let text = checked_text(run(&["-args", "-G1", "-a", "-XMP:All"], sidecar)?)?;
    Ok(text
        .lines()
        .map(|line| {
            let line = line.strip_prefix('-').unwrap_or(line);
            match line.split_once('=') {
                Some((key, value)) => format!("{key}: {value}"),
                None => line.to_string(),
            }
        })
        .collect::<Vec<_>>()
        .join("\n"))
}

fn run(args: &[&str], file: &Path) -> Result<Output, String> {
    Command::new("exiftool")
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
