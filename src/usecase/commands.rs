use crate::ports::{Editor, FileSystem, GlobExpander, Output, SidecarStore};
use std::path::{Path, PathBuf};

use super::{clear, edit, export, set, show};

/// Options controlling which properties a show command reads.
pub struct ShowOptions<'a> {
    all: bool,
    basic: bool,
    verbose: bool,
    key: Option<&'a str>,
}

impl<'a> ShowOptions<'a> {
    /// Creates show options from the selected CLI display mode.
    pub fn new(all: bool, basic: bool, verbose: bool, key: Option<&'a str>) -> Self {
        Self {
            all,
            basic,
            verbose,
            key,
        }
    }
}

/// Checks metadata-tool availability before a command runs.
///
/// Reports a fatal error through `output` and returns `false` when the tool is
/// unavailable; otherwise returns `true` without producing output.
pub fn ensure_available(tool: &impl SidecarStore, output: &mut impl Output) -> bool {
    match tool.ensure_available() {
        Ok(()) => true,
        Err(error) => {
            output.fatal(&format!("pg: {error}"));
            false
        }
    }
}

/// Expands export patterns, exports every match, and reports each result.
///
/// Returns `true` if at least one target fails. When `force` is set, existing
/// sidecars may be replaced as part of the export.
pub fn export_targets(
    patterns: &[String],
    force: bool,
    tool: &impl SidecarStore,
    filesystem: &impl FileSystem,
    globber: &impl GlobExpander,
    output: &mut impl Output,
) -> bool {
    let mut failed = false;
    for image in expand_all(patterns, globber) {
        match export(&image, force, tool, filesystem, output) {
            Ok(message) => output.success(&image, &message),
            Err(error) => {
                output.error(&image, &error);
                failed = true;
            }
        }
    }
    failed
}

/// Expands show patterns, displays each result, and continues after errors.
///
/// Returns `true` if at least one target fails; failures are reported through
/// `output` while processing continues for the remaining targets.
pub fn show_targets(
    patterns: &[String],
    options: ShowOptions<'_>,
    tool: &impl SidecarStore,
    filesystem: &impl FileSystem,
    globber: &impl GlobExpander,
    output: &mut impl Output,
) -> bool {
    let targets = expand_all(patterns, globber);
    output.begin_show(targets.len());
    let mut failed = false;
    for target in targets {
        match show(
            &target,
            options.all,
            options.basic,
            options.verbose,
            options.key,
            tool,
            filesystem,
        ) {
            Ok(text) => output.show(&target, &text),
            Err(error) => {
                output.error(&target, &error);
                failed = true;
            }
        }
    }
    failed
}

/// Sets a property on one sidecar and reports any error through the output port.
///
/// Returns `true` if the operation fails. The target must resolve to an
/// existing XMP sidecar.
pub fn set_target(
    target: &Path,
    key: &str,
    value: &str,
    tool: &impl SidecarStore,
    filesystem: &impl FileSystem,
    output: &mut impl Output,
) -> bool {
    report_silently(target, set(target, key, value, tool, filesystem), output)
}

/// Clears a property on one sidecar and reports any error through the output port.
///
/// Returns `true` if the operation fails. The target must resolve to an
/// existing XMP sidecar.
pub fn clear_target(
    target: &Path,
    key: &str,
    tool: &impl SidecarStore,
    filesystem: &impl FileSystem,
    output: &mut impl Output,
) -> bool {
    report_silently(target, clear(target, key, tool, filesystem), output)
}

/// Opens a sidecar in the selected editor and reports any error.
///
/// Returns `true` if the target cannot be resolved or the editor fails.
pub fn edit_target(
    target: &Path,
    editor: &impl Editor,
    filesystem: &impl FileSystem,
    output: &mut impl Output,
) -> bool {
    report_silently(target, edit(target, editor, filesystem), output)
}

fn report_silently<T>(target: &Path, result: Result<T, String>, output: &mut impl Output) -> bool {
    if let Err(error) = result {
        output.error(target, &error);
        true
    } else {
        false
    }
}

fn expand_all(patterns: &[String], globber: &impl GlobExpander) -> Vec<PathBuf> {
    patterns
        .iter()
        .flat_map(|pattern| globber.expand(pattern))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::{
        ShowOptions, clear_target, edit_target, ensure_available, export_targets, set_target,
        show_targets,
    };
    use crate::ports::{GlobExpander, Logger, Output};
    use crate::usecase::test_helpers::{TestEditor, TestFileSystem, TestSidecarStore};
    use std::path::{Path, PathBuf};

    #[derive(Default)]
    struct TestOutput {
        events: Vec<String>,
    }

    impl Logger for TestOutput {
        fn warn(&mut self, message: &str) {
            self.events.push(format!("warning:{message}"));
        }
    }

    impl Output for TestOutput {
        fn begin_show(&mut self, target_count: usize) {
            self.events.push(format!("begin:{target_count}"));
        }

        fn show(&mut self, target: &Path, text: &str) {
            self.events
                .push(format!("show:{}:{text}", target.display()));
        }

        fn success(&mut self, target: &Path, message: &str) {
            self.events
                .push(format!("success:{}:{message}", target.display()));
        }

        fn error(&mut self, target: &Path, message: &str) {
            self.events
                .push(format!("error:{}:{message}", target.display()));
        }

        fn fatal(&mut self, message: &str) {
            self.events.push(format!("fatal:{message}"));
        }
    }

    struct TestGlob;

    impl GlobExpander for TestGlob {
        fn expand(&self, pattern: &str) -> Vec<PathBuf> {
            match pattern {
                "*.xmp" => vec![PathBuf::from("one.xmp"), PathBuf::from("two.xmp")],
                "*.jpg" => vec![PathBuf::from("one.jpg"), PathBuf::from("two.jpg")],
                other => vec![PathBuf::from(other)],
            }
        }
    }

    #[test]
    fn ensure_available_reports_fatal_errors() {
        let mut output = TestOutput::default();
        let unavailable = TestSidecarStore {
            ensure_error: Some("exiftool unavailable".into()),
            ..Default::default()
        };
        assert!(!ensure_available(&unavailable, &mut output));
        assert_eq!(output.events, ["fatal:pg: exiftool unavailable"]);
    }

    #[test]
    fn ensure_available_succeeds_without_reporting() {
        let mut output = TestOutput::default();
        assert!(ensure_available(&TestSidecarStore::default(), &mut output));
        assert!(output.events.is_empty());
    }

    #[test]
    fn show_targets_expands_patterns_and_continues_after_errors() {
        let filesystem = TestFileSystem::default();
        filesystem.add_file(Path::new("one.xmp"));
        let store = TestSidecarStore {
            read_error: Some("read failed".into()),
            ..Default::default()
        };
        let mut output = TestOutput::default();

        assert!(show_targets(
            &["*.xmp".into()],
            ShowOptions::new(false, false, false, None),
            &store,
            &filesystem,
            &TestGlob,
            &mut output
        ));
        assert_eq!(
            output.events,
            [
                "begin:2",
                "error:one.xmp:read failed",
                "error:two.xmp:sidecar not found: two.xmp"
            ]
        );
    }

    #[test]
    fn show_targets_reports_successful_results() {
        let filesystem = TestFileSystem::default();
        filesystem.add_file(Path::new("one.xmp"));
        filesystem.add_file(Path::new("two.xmp"));
        let store = TestSidecarStore::default();
        let mut output = TestOutput::default();

        assert!(!show_targets(
            &["*.xmp".into()],
            ShowOptions::new(false, false, false, None),
            &store,
            &filesystem,
            &TestGlob,
            &mut output
        ));
        assert_eq!(
            output.events,
            [
                "begin:2",
                "show:one.xmp:properties",
                "show:two.xmp:properties"
            ]
        );
    }

    #[test]
    fn export_targets_reports_success_and_error_for_each_match() {
        let filesystem = TestFileSystem::default();
        filesystem.add_file(Path::new("one.jpg"));
        filesystem.add_file(Path::new("two.jpg"));
        let store = TestSidecarStore {
            filesystem: filesystem.clone(),
            export_creates_file: true,
            ..Default::default()
        };
        let mut output = TestOutput::default();

        assert!(export_targets(
            &["*.jpg".into(), "unsupported.txt".into()],
            false,
            &store,
            &filesystem,
            &TestGlob,
            &mut output
        ));
        assert_eq!(
            output.events,
            [
                "success:one.jpg:exported to one.xmp",
                "success:two.jpg:exported to two.xmp",
                "error:unsupported.txt:export requires a supported image path"
            ]
        );
    }

    #[test]
    fn export_targets_logs_forced_basename_collisions() {
        let filesystem = TestFileSystem::default();
        let image = Path::new("/photos/photo.jpg");
        filesystem.add_file(image);
        filesystem.add_file(Path::new("/photos/photo.NEF"));
        let store = TestSidecarStore {
            filesystem: filesystem.clone(),
            export_creates_file: true,
            ..Default::default()
        };
        let mut output = TestOutput::default();

        assert!(!export_targets(
            &["/photos/photo.jpg".into()],
            true,
            &store,
            &filesystem,
            &TestGlob,
            &mut output
        ));
        assert_eq!(
            output.events,
            [
                "warning:sidecar basename is ambiguous with /photos/photo.NEF; proceeding because --force was specified",
                "success:/photos/photo.jpg:exported to /photos/photo.xmp"
            ]
        );
    }

    #[test]
    fn mutation_and_edit_targets_report_errors() {
        let filesystem = TestFileSystem::default();
        let store = TestSidecarStore::default();
        let editor = TestEditor::default();
        let mut output = TestOutput::default();

        assert!(set_target(
            Path::new("missing.xmp"),
            "XMP-dc:Description",
            "value",
            &store,
            &filesystem,
            &mut output
        ));
        assert!(clear_target(
            Path::new("missing.xmp"),
            "XMP-dc:Description",
            &store,
            &filesystem,
            &mut output
        ));
        assert!(edit_target(
            Path::new("missing.xmp"),
            &editor,
            &filesystem,
            &mut output
        ));
        assert_eq!(output.events.len(), 3);
        assert!(
            output
                .events
                .iter()
                .all(|event| event.contains("sidecar not found"))
        );
    }

    #[test]
    fn successful_mutations_and_edit_do_not_write_errors() {
        let filesystem = TestFileSystem::default();
        let sidecar = Path::new("photo.xmp");
        filesystem.add_file(sidecar);
        let store = TestSidecarStore::default();
        let editor = TestEditor::default();
        let mut output = TestOutput::default();

        assert!(!set_target(
            sidecar,
            "XMP-dc:Description",
            "value",
            &store,
            &filesystem,
            &mut output
        ));
        assert!(!clear_target(
            sidecar,
            "XMP-dc:Description",
            &store,
            &filesystem,
            &mut output
        ));
        assert!(!edit_target(sidecar, &editor, &filesystem, &mut output));
        assert!(output.events.is_empty());
    }
}
