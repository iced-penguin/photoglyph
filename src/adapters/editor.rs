use crate::ports::Editor;
use std::env;
use std::path::Path;
use std::process::Command;

/// Launches the editor named by `$VISUAL` or `$EDITOR`.
#[derive(Default)]
pub struct EnvEditor;

impl Editor for EnvEditor {
    fn edit(&self, sidecar: &Path) -> Result<(), String> {
        let editor = env::var("VISUAL")
            .ok()
            .filter(|s| !s.trim().is_empty())
            .or_else(|| env::var("EDITOR").ok().filter(|s| !s.trim().is_empty()))
            .ok_or("set $VISUAL or $EDITOR to an editor command")?;
        run_editor(sidecar, &editor)
    }
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
    #[cfg(unix)]
    use super::EnvEditor;
    use super::run_editor;
    #[cfg(unix)]
    use crate::ports::Editor;
    use std::path::Path;

    #[cfg(unix)]
    #[test]
    fn run_editor_succeeds_for_a_successful_command() {
        let sidecar = Path::new("/photos/photo.xmp");
        assert_eq!(run_editor(sidecar, "true"), Ok(()));
    }

    #[cfg(unix)]
    #[test]
    fn run_editor_reports_a_nonzero_exit_status() {
        let sidecar = Path::new("/photos/photo.xmp");
        assert!(
            run_editor(sidecar, "false")
                .unwrap_err()
                .contains("editor failed")
        );
    }

    #[test]
    fn run_editor_rejects_an_empty_command() {
        let sidecar = Path::new("/photos/photo.xmp");
        assert!(
            run_editor(sidecar, "")
                .unwrap_err()
                .contains("editor command is empty")
        );
    }

    #[cfg(unix)]
    #[test]
    fn run_editor_reports_when_the_command_cannot_be_started() {
        let sidecar = Path::new("/photos/photo.xmp");
        assert!(
            run_editor(sidecar, "pg-no-such-editor-command")
                .unwrap_err()
                .contains("cannot start editor")
        );
    }

    #[cfg(unix)]
    #[test]
    fn edit_requires_an_editor_setting() {
        let _lock = EDITOR_ENV_LOCK.lock().unwrap();
        let sidecar = Path::new("/photos/photo.xmp");
        let _environment = EditorEnv::set(None, None);
        assert!(
            EnvEditor
                .edit(sidecar)
                .unwrap_err()
                .contains("set $VISUAL or $EDITOR")
        );
    }

    #[cfg(unix)]
    #[test]
    fn edit_falls_back_to_editor_when_visual_is_blank() {
        let _lock = EDITOR_ENV_LOCK.lock().unwrap();
        let sidecar = Path::new("/photos/photo.xmp");
        let _environment = EditorEnv::set(Some("  "), Some("true"));
        assert_eq!(EnvEditor.edit(sidecar), Ok(()));
    }

    #[cfg(unix)]
    #[test]
    fn edit_prefers_visual_over_editor() {
        let _lock = EDITOR_ENV_LOCK.lock().unwrap();
        let sidecar = Path::new("/photos/photo.xmp");
        let _environment = EditorEnv::set(Some("false"), Some("true"));
        assert!(
            EnvEditor
                .edit(sidecar)
                .unwrap_err()
                .contains("editor failed")
        );
    }

    #[cfg(unix)]
    #[test]
    fn editor_env_restores_previous_values_on_drop() {
        let _lock = EDITOR_ENV_LOCK.lock().unwrap();
        let _outer = EditorEnv::set(Some("visual-editor"), Some("plain-editor"));
        drop(EditorEnv::set(None, None));
        assert_eq!(std::env::var_os("VISUAL"), Some("visual-editor".into()));
        assert_eq!(std::env::var_os("EDITOR"), Some("plain-editor".into()));
    }

    #[cfg(unix)]
    static EDITOR_ENV_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());

    #[cfg(unix)]
    struct EditorEnv {
        visual: Option<std::ffi::OsString>,
        editor: Option<std::ffi::OsString>,
    }

    #[cfg(unix)]
    impl EditorEnv {
        fn set(visual: Option<&str>, editor: Option<&str>) -> Self {
            let old = Self {
                visual: std::env::var_os("VISUAL"),
                editor: std::env::var_os("EDITOR"),
            };
            unsafe {
                match visual {
                    Some(value) => std::env::set_var("VISUAL", value),
                    None => std::env::remove_var("VISUAL"),
                }
                match editor {
                    Some(value) => std::env::set_var("EDITOR", value),
                    None => std::env::remove_var("EDITOR"),
                }
            }
            old
        }
    }

    #[cfg(unix)]
    impl Drop for EditorEnv {
        fn drop(&mut self) {
            unsafe {
                match &self.visual {
                    Some(value) => std::env::set_var("VISUAL", value),
                    None => std::env::remove_var("VISUAL"),
                }
                match &self.editor {
                    Some(value) => std::env::set_var("EDITOR", value),
                    None => std::env::remove_var("EDITOR"),
                }
            }
        }
    }
}
