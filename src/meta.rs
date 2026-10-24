use std::env;
use std::fs;
use std::io::ErrorKind;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

pub const DEFAULT_COMMENT: &str = "XMP-dc:Description";
const EXTENSIONS: &[&str] = &["jpg", "jpeg", "png", "nef", "tif", "tiff", "dng"];
const BASIC_KEYS: &[&str] = &[
    "XMP-dc:Description",
    "XMP-dc:Creator",
    "XMP-dc:Rights",
    "XMP-xmp:CreateDate",
    "XMP-xmp:ModifyDate",
    "XMP-xmp:Rating",
    "XMP-photoshop:DateCreated",
];
const INSTALL_HINT: &str = "exiftool not found. Install it and try again:
  macOS:           brew install exiftool
  Debian/Ubuntu:   sudo apt install libimage-exiftool-perl
  other:           https://exiftool.org";

pub fn ensure_exiftool() -> Result<(), String> {
    match Command::new("exiftool").arg("-ver").output() {
        Ok(out) if out.status.success() => Ok(()),
        Ok(out) => Err(output_error(&out)),
        Err(error) if error.kind() == ErrorKind::NotFound => Err(INSTALL_HINT.into()),
        Err(error) => Err(format!("cannot run exiftool: {error}")),
    }
}

pub fn export(image: &Path, force: bool) -> Result<String, String> {
    if !is_image(image) {
        return Err("export requires a supported image path".into());
    }
    let image = canonical(image)?;
    if let Some(collision) = find_stem_collision(&image)? {
        if force {
            eprintln!(
                "warning: sidecar basename is ambiguous with {}; proceeding because --force was specified",
                collision.display()
            );
        } else {
            return Err(stem_collision_error(&collision));
        }
    }
    let sidecar = companion(&image);
    if sidecar.exists() && !force {
        return Err(format!(
            "sidecar already exists: {}; use --force to replace it",
            sidecar.display()
        ));
    }
    let temporary = temporary_sidecar(&sidecar);
    let out = Command::new("exiftool")
        .args(["-charset", "utf8", "-tagsFromFile"])
        .arg(&image)
        .arg("-o")
        .arg(&temporary)
        .arg(&image)
        .output()
        .map_err(|error| format!("cannot run exiftool: {error}"))?;
    if !out.status.success() {
        let _ = fs::remove_file(&temporary);
        return Err(output_error(&out));
    }
    if !temporary.is_file() {
        return Err("exiftool reported success but did not create an XMP sidecar".into());
    }
    let installed = if force {
        fs::rename(&temporary, &sidecar)
    } else {
        fs::hard_link(&temporary, &sidecar).and_then(|()| fs::remove_file(&temporary))
    };
    if let Err(error) = installed {
        let _ = fs::remove_file(&temporary);
        return Err(if sidecar.exists() && !force {
            format!(
                "sidecar already exists: {}; use --force to replace it",
                sidecar.display()
            )
        } else {
            format!("cannot install sidecar {}: {error}", sidecar.display())
        });
    }
    Ok(format!("exported to {}", sidecar.display()))
}

pub fn show(
    target: &Path,
    all: bool,
    basic: bool,
    verbose: bool,
    key: Option<&str>,
) -> Result<String, String> {
    let sidecar = resolve_xmp_target(target)?;
    if all {
        return show_all(&sidecar, verbose);
    }
    let keys: Vec<&str> = match key {
        Some(key) => {
            validate_key(key)?;
            vec![key]
        }
        None if basic => BASIC_KEYS.to_vec(),
        None => vec![DEFAULT_COMMENT],
    };
    if verbose {
        let selectors: Vec<String> = keys.iter().map(|key| format!("-{key}")).collect();
        let mut args = vec!["-G1", "-a"];
        args.extend(selectors.iter().map(String::as_str));
        return checked_text(run(&args, &sidecar)?);
    }
    let formats: Vec<String> = keys
        .iter()
        .map(|key| format!("{key}: ${{{key}}}"))
        .collect();
    let mut args = Vec::new();
    for format in &formats {
        args.extend(["-p", format]);
    }
    checked_text(run(&args, &sidecar)?)
}

fn show_all(sidecar: &Path, verbose: bool) -> Result<String, String> {
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

pub fn set(target: &Path, key: &str, value: &str) -> Result<String, String> {
    write_property(target, key, value)
}

pub fn clear(target: &Path, key: &str) -> Result<String, String> {
    validate_key(key)?;
    let sidecar = resolve_xmp_target(target)?;
    let assignment = format!("-{key}^=");
    run_mutation(&sidecar, &assignment)
        .map(|updated| if updated { "cleared" } else { "unchanged" }.into())
}

pub fn edit(target: &Path) -> Result<String, String> {
    let sidecar = resolve_xmp_target(target)?;
    let editor = env::var("VISUAL")
        .ok()
        .filter(|s| !s.trim().is_empty())
        .or_else(|| env::var("EDITOR").ok().filter(|s| !s.trim().is_empty()))
        .ok_or("set $VISUAL or $EDITOR to an editor command")?;
    let mut parts = editor.split_whitespace();
    let executable = parts.next().ok_or("editor command is empty")?;
    let status = Command::new(executable)
        .args(parts)
        .arg(&sidecar)
        .status()
        .map_err(|error| format!("cannot start editor: {error}"))?;
    if status.success() {
        Ok("editor exited successfully".into())
    } else {
        Err(format!("editor failed ({status})"))
    }
}

fn write_property(target: &Path, key: &str, value: &str) -> Result<String, String> {
    validate_key(key)?;
    let sidecar = resolve_xmp_target(target)?;
    let assignment = format!("-{key}={value}");
    run_mutation(&sidecar, &assignment)
        .map(|updated| if updated { "updated" } else { "unchanged" }.into())
}

fn run_mutation(sidecar: &Path, assignment: &str) -> Result<bool, String> {
    let out = run(&["-overwrite_original", assignment], sidecar)?;
    if !out.status.success() {
        return Err(output_error(&out));
    }
    let stdout = String::from_utf8_lossy(&out.stdout);
    Ok(stdout.contains("1 image files updated"))
}

/// Accepts an .xmp path or a bare basename (`photo` means `photo.xmp`); image paths are rejected.
fn resolve_xmp_target(target: &Path) -> Result<PathBuf, String> {
    if is_image(target) {
        return Err("expected an .xmp file or a basename (e.g. `photo` for photo.xmp)".into());
    }
    let path = if is_xmp(target) {
        target.to_path_buf()
    } else {
        let mut name = target.as_os_str().to_os_string();
        name.push(".xmp");
        PathBuf::from(name)
    };
    if !path.is_file() {
        return Err(format!("sidecar not found: {}", path.display()));
    }
    canonical(&path).and_then(require_xmp)
}

fn require_xmp(path: PathBuf) -> Result<PathBuf, String> {
    if is_xmp(&path) {
        Ok(path)
    } else {
        Err("resolved file is not an .xmp sidecar".into())
    }
}

fn canonical(path: &Path) -> Result<PathBuf, String> {
    fs::canonicalize(path).map_err(|error| match error.kind() {
        ErrorKind::NotFound => format!("file not found: {}", path.display()),
        _ => error.to_string(),
    })
}

fn companion(image: &Path) -> PathBuf {
    image.with_extension("xmp")
}

fn find_stem_collision(image: &Path) -> Result<Option<PathBuf>, String> {
    let Some(stem) = image.file_stem() else {
        return Err("image path has no filename".into());
    };
    let parent = image.parent().unwrap_or_else(|| Path::new("."));
    let entries = fs::read_dir(parent).map_err(|error| error.to_string())?;
    for entry in entries.flatten() {
        let path = entry.path();
        if path == image || !is_image(&path) {
            continue;
        }
        if path.file_stem().is_some_and(|other| other == stem) {
            return Ok(Some(path));
        }
    }
    Ok(None)
}

fn stem_collision_error(collision: &Path) -> String {
    format!(
        "sidecar basename is ambiguous with {}; refusing unsafe operation",
        collision.display()
    )
}

fn temporary_sidecar(sidecar: &Path) -> PathBuf {
    let name = sidecar.file_name().unwrap_or_default().to_string_lossy();
    let mut index = 0u32;
    loop {
        let candidate =
            sidecar.with_file_name(format!(".{name}.{}.{}.tmp.xmp", std::process::id(), index));
        if !candidate.exists() {
            return candidate;
        }
        index += 1;
    }
}

fn validate_key(key: &str) -> Result<(), String> {
    if key.is_empty()
        || key.starts_with('-')
        || !key.contains(':')
        || !key
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b == b':' || b == b'_' || b == b'-')
    {
        return Err("invalid XMP key; use ExifTool syntax such as XMP-dc:Description".into());
    }
    Ok(())
}

fn is_image(path: &Path) -> bool {
    path.extension()
        .and_then(|ext| ext.to_str())
        .is_some_and(|ext| EXTENSIONS.contains(&ext.to_ascii_lowercase().as_str()))
}

fn is_xmp(path: &Path) -> bool {
    path.extension()
        .and_then(|ext| ext.to_str())
        .is_some_and(|ext| ext.eq_ignore_ascii_case("xmp"))
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
    if !out.status.success() {
        return Err(output_error(&out));
    }
    Ok(String::from_utf8_lossy(&out.stdout).trim_end().to_string())
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
    use super::{find_stem_collision, is_image, is_xmp, resolve_xmp_target, validate_key};
    use std::fs;
    use std::path::Path;

    #[test]
    fn accepts_supported_image_and_xmp_extensions_case_insensitively() {
        assert!(is_image(Path::new("photo.NEF")));
        assert!(is_image(Path::new("photo.tiff")));
        assert!(is_xmp(Path::new("photo.XMP")));
        assert!(!is_image(Path::new("photo.txt")));
    }

    #[test]
    fn rejects_image_paths_for_xmp_targets() {
        assert!(
            resolve_xmp_target(Path::new("photo.jpg"))
                .unwrap_err()
                .contains("expected an .xmp file or a basename")
        );
    }

    #[test]
    fn validates_exiftool_keys_as_individual_arguments() {
        assert!(validate_key("XMP-dc:Description").is_ok());
        assert!(validate_key("XMP-dc:Description=value").is_err());
        assert!(validate_key("-XMP-dc:Description").is_err());
        assert!(validate_key("XMP-dc:Description\n-other").is_err());
    }

    #[test]
    fn rejects_ambiguous_image_basenames() {
        let directory = std::env::temp_dir().join(format!("pg-collision-{}", std::process::id()));
        let _ = fs::remove_dir_all(&directory);
        fs::create_dir_all(&directory).unwrap();
        let jpeg = directory.join("photo.jpg");
        fs::write(&jpeg, b"jpeg").unwrap();
        fs::write(directory.join("photo.NEF"), b"raw").unwrap();
        assert_eq!(
            find_stem_collision(&jpeg).unwrap(),
            Some(directory.join("photo.NEF"))
        );
        fs::remove_dir_all(directory).unwrap();
    }
}
