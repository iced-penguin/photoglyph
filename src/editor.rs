use crate::port::Editor;
use std::env;
use std::path::Path;
use std::process::Command;

/// Launches the editor named by `$VISUAL` or `$EDITOR`.
#[derive(Default)]
pub struct EnvEditor;

impl Editor for EnvEditor {
    fn open(&self, sidecar: &Path) -> Result<(), String> {
        let editor = configured_editor(env::var("VISUAL").ok(), env::var("EDITOR").ok())?;
        run_editor(sidecar, &editor)
    }
}

fn configured_editor(visual: Option<String>, editor: Option<String>) -> Result<String, String> {
    visual
        .filter(|s| !s.trim().is_empty())
        .or_else(|| editor.filter(|s| !s.trim().is_empty()))
        .ok_or_else(|| "set $VISUAL or $EDITOR to an editor command".to_string())
}

fn run_editor(sidecar: &Path, editor: &str) -> Result<(), String> {
    let mut parts = editor.split_whitespace();
    let executable = parts.next().ok_or("editor command is empty")?;
    let status = Command::new(executable)
        .args(parts)
        .arg(sidecar)
        .status()
        .map_err(|error| format!("cannot start editor: {error}"))?;
    if status.success() {
        Ok(())
    } else {
        Err(format!("editor failed ({status})"))
    }
}

#[cfg(test)]
mod tests {
    use super::{configured_editor, run_editor};
    use std::path::Path;

    #[test]
    fn requires_an_editor_setting() {
        assert!(
            configured_editor(None, None)
                .unwrap_err()
                .contains("set $VISUAL or $EDITOR")
        );
    }

    #[test]
    fn falls_back_to_editor_when_visual_is_blank() {
        assert_eq!(
            configured_editor(Some("  ".into()), Some("nano".into())),
            Ok("nano".into())
        );
    }

    #[test]
    fn prefers_visual_over_editor() {
        assert_eq!(
            configured_editor(Some("vim".into()), Some("nano".into())),
            Ok("vim".into())
        );
    }

    #[test]
    fn rejects_an_empty_command() {
        assert!(
            run_editor(Path::new("/photos/photo.xmp"), "")
                .unwrap_err()
                .contains("editor command is empty")
        );
    }

    #[cfg(unix)]
    #[test]
    fn succeeds_for_a_successful_command() {
        assert_eq!(run_editor(Path::new("/photos/photo.xmp"), "true"), Ok(()));
    }

    #[cfg(unix)]
    #[test]
    fn reports_a_nonzero_exit_status() {
        assert!(
            run_editor(Path::new("/photos/photo.xmp"), "false")
                .unwrap_err()
                .contains("editor failed")
        );
    }

    #[cfg(unix)]
    #[test]
    fn reports_when_the_command_cannot_be_started() {
        assert!(
            run_editor(Path::new("/photos/photo.xmp"), "pg-no-such-editor-command")
                .unwrap_err()
                .contains("cannot start editor")
        );
    }
}
