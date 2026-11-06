use crate::ports::{FileSystem, Logger, SidecarStore};
use std::path::{Path, PathBuf};

use super::shared::{canonical, is_image};

pub fn export(
    image: &Path,
    force: bool,
    tool: &impl SidecarStore,
    filesystem: &impl FileSystem,
    logger: &mut impl Logger,
) -> Result<String, String> {
    if !is_image(image) {
        return Err("export requires a supported image path".into());
    }
    let image = canonical(image, filesystem)?;
    if let Some(collision) = find_stem_collision(&image, filesystem)? {
        if force {
            logger.warn(&format!(
                "sidecar basename is ambiguous with {}; proceeding because --force was specified",
                collision.display()
            ));
        } else {
            return Err(stem_collision_error(&collision));
        }
    }
    let sidecar = companion(&image);
    if filesystem.exists(&sidecar) && !force {
        return Err(format!(
            "sidecar already exists: {}; use --force to replace it",
            sidecar.display()
        ));
    }
    let temporary = temporary_sidecar(&sidecar, filesystem);
    if let Err(error) = tool.export_to(&image, &temporary) {
        let _ = filesystem.remove_file(&temporary);
        return Err(error);
    }
    if !filesystem.is_file(&temporary) {
        return Err("exiftool reported success but did not create an XMP sidecar".into());
    }
    if let Err(error) = filesystem.install_sidecar(&temporary, &sidecar, force) {
        let _ = filesystem.remove_file(&temporary);
        return Err(if filesystem.exists(&sidecar) && !force {
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

fn companion(image: &Path) -> PathBuf {
    image.with_extension("xmp")
}

pub(super) fn find_stem_collision(
    image: &Path,
    filesystem: &impl FileSystem,
) -> Result<Option<PathBuf>, String> {
    let Some(stem) = image.file_stem() else {
        return Err("image path has no filename".into());
    };
    let parent = parent_or_current_dir(image.parent());
    let entries = filesystem
        .read_dir(parent)
        .map_err(|error| error.to_string())?;
    for path in entries {
        if path == image || !is_image(&path) {
            continue;
        }
        if path.file_stem().is_some_and(|other| other == stem) {
            return Ok(Some(path));
        }
    }
    Ok(None)
}

pub(super) fn parent_or_current_dir(parent: Option<&Path>) -> &Path {
    parent.unwrap_or_else(|| Path::new("."))
}

fn stem_collision_error(collision: &Path) -> String {
    format!(
        "sidecar basename is ambiguous with {}; refusing unsafe operation",
        collision.display()
    )
}

fn temporary_sidecar(sidecar: &Path, filesystem: &impl FileSystem) -> PathBuf {
    let name = sidecar.file_name().unwrap_or_default().to_string_lossy();
    let mut index = 0u32;
    loop {
        let candidate =
            sidecar.with_file_name(format!(".{name}.{}.{}.tmp.xmp", std::process::id(), index));
        if !filesystem.exists(&candidate) {
            return candidate;
        }
        index += 1;
    }
}
