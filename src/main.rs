mod adapters;
mod ports;
mod usecase;

use adapters::{editor, exiftool, filesystem, glob};
use clap::{Args, Parser, Subcommand};
use ports::{GlobExpander, SidecarStore};
use std::path::{Path, PathBuf};
use std::process::ExitCode;

/// Export and edit XMP sidecars without writing metadata into image files.
#[derive(Parser)]
#[command(name = "pg", version)]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Export image metadata to a companion .xmp sidecar.
    Export {
        /// Explicitly replace an existing sidecar.
        #[arg(short, long)]
        force: bool,
        /// Image files to export; quoted glob patterns such as "*.jpg" are allowed.
        #[arg(required = true, value_name = "TARGETS")]
        targets: Vec<String>,
    },
    /// Show properties.
    Show {
        #[command(flatten)]
        display: DisplayOptions,
        /// Use a more detailed output format.
        #[arg(short, long)]
        verbose: bool,
        /// .xmp sidecars or basenames; quoted glob patterns such as "*.xmp" are allowed.
        #[arg(required = true, value_name = "TARGETS")]
        targets: Vec<String>,
    },
    /// Set a property value.
    Set {
        #[command(flatten)]
        property: PropertyOption,
        /// .xmp sidecar or basename (a single file; globs are not expanded).
        #[arg(value_name = "TARGET")]
        target: String,
        /// Text to store.
        #[arg(allow_hyphen_values = true)]
        value: String,
    },
    /// Set a property to a visually empty value.
    Clear {
        #[command(flatten)]
        property: PropertyOption,
        /// .xmp sidecar or basename (a single file; globs are not expanded).
        #[arg(value_name = "TARGET")]
        target: String,
    },
    /// Open the resolved XMP sidecar in $VISUAL or $EDITOR.
    Edit {
        /// .xmp sidecar or basename (a single file; globs are not expanded).
        #[arg(value_name = "TARGET")]
        target: String,
    },
}

#[derive(Args)]
#[group(multiple = false)]
struct DisplayOptions {
    /// Show a concise set of common XMP properties.
    #[arg(short, long)]
    basic: bool,
    /// Show every XMP property.
    #[arg(short, long)]
    all: bool,
    /// Show one ExifTool XMP key (default without options: XMP-dc:Description).
    #[arg(short, long)]
    key: Option<String>,
}

#[derive(Args)]
struct PropertyOption {
    /// ExifTool XMP key, e.g. XMP-dc:Description (default: XMP-dc:Description).
    #[arg(short, long)]
    key: Option<String>,
}

fn main() -> ExitCode {
    let cli = match Cli::try_parse() {
        Ok(cli) => cli,
        Err(error) => {
            let _ = error.print();
            return if error.use_stderr() {
                ExitCode::from(1)
            } else {
                ExitCode::SUCCESS
            };
        }
    };
    let exiftool = exiftool::ExifTool::default();
    let editor = editor::EnvEditor;
    let filesystem = filesystem::LocalFileSystem;
    let globber = glob::ShellGlob;
    if let Err(error) = exiftool.ensure_available() {
        eprintln!("pg: {error}");
        return ExitCode::from(1);
    }

    let mut failed = false;
    match cli.command {
        Command::Export { force, targets } => {
            for image in expand_all(&targets, &globber) {
                report(
                    image.display(),
                    usecase::export(&image, force, &exiftool, &filesystem),
                    &mut failed,
                );
            }
        }
        Command::Show {
            display,
            verbose,
            targets,
        } => {
            let key = display.key.as_deref();
            let expanded = expand_all(&targets, &globber);
            let labeled = expanded.len() > 1;
            let mut printed = false;
            for target in expanded {
                let result = usecase::show(
                    &target,
                    display.all,
                    display.basic,
                    verbose,
                    key,
                    &exiftool,
                    &filesystem,
                );
                match result {
                    Ok(text) => {
                        if let Some(output) =
                            format_show_result(&target, &text, labeled, &mut printed)
                        {
                            print!("{output}");
                        }
                    }
                    Err(error) => {
                        eprintln!("{}: error: {error}", target.display());
                        failed = true;
                    }
                }
            }
        }
        Command::Set {
            property,
            target,
            value,
        } => {
            let key = property.key.as_deref().unwrap_or(usecase::DEFAULT_COMMENT);
            report_silently(
                &target,
                usecase::set(target.as_ref(), key, &value, &exiftool, &filesystem),
                &mut failed,
            );
        }
        Command::Clear { property, target } => {
            let key = property.key.as_deref().unwrap_or(usecase::DEFAULT_COMMENT);
            report_silently(
                &target,
                usecase::clear(target.as_ref(), key, &exiftool, &filesystem),
                &mut failed,
            );
        }
        Command::Edit { target } => report_silently(
            &target,
            usecase::edit(target.as_ref(), &editor, &filesystem),
            &mut failed,
        ),
    }
    if failed {
        ExitCode::from(2)
    } else {
        ExitCode::SUCCESS
    }
}

fn expand_all(patterns: &[String], globber: &impl GlobExpander) -> Vec<PathBuf> {
    patterns
        .iter()
        .flat_map(|pattern| globber.expand(pattern))
        .collect()
}

fn report<T: std::fmt::Display>(
    target: impl std::fmt::Display,
    result: Result<T, String>,
    failed: &mut bool,
) {
    match result {
        Ok(message) => println!("{target}: {message}"),
        Err(error) => {
            eprintln!("{target}: error: {error}");
            *failed = true;
        }
    }
}

fn report_silently<T>(
    target: impl std::fmt::Display,
    result: Result<T, String>,
    failed: &mut bool,
) {
    if let Err(error) = result {
        eprintln!("{target}: error: {error}");
        *failed = true;
    }
}

fn format_show_output(target: &Path, text: &str, labeled: bool) -> String {
    if labeled {
        format!("== {} ==\n{text}", target.display())
    } else {
        text.to_string()
    }
}

fn format_show_result(
    target: &Path,
    text: &str,
    labeled: bool,
    printed: &mut bool,
) -> Option<String> {
    if text.is_empty() && !labeled {
        return None;
    }

    let separator = if labeled && *printed { "\n" } else { "" };
    *printed = true;
    let output = format_show_output(target, text, labeled);
    if labeled && text.is_empty() {
        Some(format!("{separator}{output}"))
    } else {
        Some(format!("{separator}{output}\n"))
    }
}

#[cfg(test)]
mod tests {
    use super::{Cli, Command, expand_all, format_show_output, format_show_result};
    use crate::ports::GlobExpander;
    use clap::{Parser, error::ErrorKind};
    use rstest::rstest;
    use std::path::{Path, PathBuf};

    struct FakeGlob;

    impl GlobExpander for FakeGlob {
        fn expand(&self, pattern: &str) -> Vec<PathBuf> {
            match pattern {
                "*.jpg" => vec![PathBuf::from("a.jpg"), PathBuf::from("b.jpg")],
                "*.none" => vec![],
                other => vec![PathBuf::from(other)],
            }
        }
    }

    #[rstest]
    #[case::multiple(
        "a/x.xmp",
        "XMP-dc:Description: A",
        true,
        "== a/x.xmp ==\nXMP-dc:Description: A"
    )]
    #[case::single("a/x.xmp", "XMP-dc:Description: A", false, "XMP-dc:Description: A")]
    fn formats_show_output(
        #[case] path: &str,
        #[case] text: &str,
        #[case] labeled: bool,
        #[case] expected: &str,
    ) {
        assert_eq!(format_show_output(Path::new(path), text, labeled), expected);
    }

    #[test]
    fn formats_multiple_show_results_including_empty_results() {
        let results = [
            (Path::new("empty-before.xmp"), ""),
            (Path::new("a.xmp"), "XMP-dc:Description: A"),
            (Path::new("empty-between.xmp"), ""),
            (Path::new("b.xmp"), "XMP-dc:Description: B"),
            (Path::new("empty-after.xmp"), ""),
        ];
        let mut printed = false;
        let mut output = String::new();

        for (target, text) in results {
            if let Some(result) = format_show_result(target, text, true, &mut printed) {
                output.push_str(&result);
            }
        }

        assert_eq!(
            output,
            "== empty-before.xmp ==\n\n== a.xmp ==\nXMP-dc:Description: A\n\n== empty-between.xmp ==\n\n== b.xmp ==\nXMP-dc:Description: B\n\n== empty-after.xmp ==\n"
        );
    }

    #[test]
    fn expand_all_flattens_expansions_in_order() {
        let patterns = ["x.nef", "*.jpg", "*.none", "y.nef"].map(String::from);
        assert_eq!(
            expand_all(&patterns, &FakeGlob),
            ["x.nef", "a.jpg", "b.jpg", "y.nef"].map(PathBuf::from)
        );
    }

    fn parse(args: &[&str]) -> Command {
        Cli::try_parse_from(args).unwrap().command
    }

    fn targets_of(command: Command) -> Vec<String> {
        match command {
            Command::Export { targets, .. } | Command::Show { targets, .. } => targets,
            _ => panic!("command has no multiple targets"),
        }
    }

    fn target_of(command: Command) -> String {
        match command {
            Command::Set { target, .. }
            | Command::Clear { target, .. }
            | Command::Edit { target } => target,
            _ => panic!("command has no single target"),
        }
    }

    fn key_of(command: Command) -> Option<String> {
        match command {
            Command::Show { display, .. } => display.key,
            Command::Set { property, .. } | Command::Clear { property, .. } => property.key,
            _ => panic!("command has no --key option"),
        }
    }

    #[rstest]
    #[case::export(&["pg", "export", "one.jpg", "two.nef"], &["one.jpg", "two.nef"])]
    #[case::show(&["pg", "show", "one.xmp", "two.xmp"], &["one.xmp", "two.xmp"])]
    fn parses_multiple_targets(#[case] args: &[&str], #[case] expected: &[&str]) {
        assert_eq!(targets_of(parse(args)), expected);
    }

    #[rstest]
    #[case::set(&["pg", "set", "photo", "text"])]
    #[case::clear(&["pg", "clear", "photo"])]
    #[case::edit(&["pg", "edit", "photo"])]
    fn parses_single_target(#[case] args: &[&str]) {
        assert_eq!(target_of(parse(args)), "photo");
    }

    #[rstest]
    #[case::show(&["pg", "show", "--key", "XMP-dc:Creator", "photo"], Some("XMP-dc:Creator"))]
    #[case::show_short(&["pg", "show", "-k", "XMP-dc:Creator", "photo"], Some("XMP-dc:Creator"))]
    #[case::show_equals(&["pg", "show", "--key=XMP-dc:Creator", "photo"], Some("XMP-dc:Creator"))]
    #[case::show_attached_short(&["pg", "show", "-kXMP-dc:Creator", "photo"], Some("XMP-dc:Creator"))]
    #[case::show_default(&["pg", "show", "photo"], None)]
    #[case::set(&["pg", "set", "--key", "XMP-dc:Creator", "photo", "text"], Some("XMP-dc:Creator"))]
    #[case::set_short(&["pg", "set", "-k", "XMP-dc:Creator", "photo", "text"], Some("XMP-dc:Creator"))]
    #[case::set_default(&["pg", "set", "photo", "text"], None)]
    #[case::clear(&["pg", "clear", "--key", "XMP-dc:Creator", "photo"], Some("XMP-dc:Creator"))]
    #[case::clear_short(&["pg", "clear", "-k", "XMP-dc:Creator", "photo"], Some("XMP-dc:Creator"))]
    #[case::clear_default(&["pg", "clear", "photo"], None)]
    fn parses_key_option(#[case] args: &[&str], #[case] expected: Option<&str>) {
        assert_eq!(key_of(parse(args)).as_deref(), expected);
    }

    #[rstest]
    #[case::without_force(&["pg", "export", "one.jpg"], false)]
    #[case::long(&["pg", "export", "--force", "one.jpg"], true)]
    #[case::short(&["pg", "export", "-f", "one.jpg"], true)]
    fn parses_export_force(#[case] args: &[&str], #[case] expected: bool) {
        let Command::Export { force, .. } = parse(args) else {
            panic!("expected export command");
        };
        assert_eq!(force, expected);
    }

    #[rstest]
    #[case::default(&["pg", "show", "one.xmp"], false, false, false)]
    #[case::basic(&["pg", "show", "--basic", "one.xmp"], true, false, false)]
    #[case::basic_short(&["pg", "show", "-b", "one.xmp"], true, false, false)]
    #[case::all(&["pg", "show", "--all", "one.xmp"], false, true, false)]
    #[case::all_short(&["pg", "show", "-a", "one.xmp"], false, true, false)]
    #[case::verbose(&["pg", "show", "--verbose", "one.xmp"], false, false, true)]
    #[case::verbose_short(&["pg", "show", "-v", "one.xmp"], false, false, true)]
    #[case::all_verbose(&["pg", "show", "--all", "--verbose", "one.xmp"], false, true, true)]
    #[case::basic_verbose(&["pg", "show", "--basic", "--verbose", "one.xmp"], true, false, true)]
    #[case::short_basic_verbose(&["pg", "show", "-b", "-v", "one.xmp"], true, false, true)]
    #[case::long_basic_short_verbose(&["pg", "show", "--basic", "-v", "one.xmp"], true, false, true)]
    #[case::short_all_verbose(&["pg", "show", "-a", "-v", "one.xmp"], false, true, true)]
    #[case::bundled_basic_verbose(&["pg", "show", "-bv", "one.xmp"], true, false, true)]
    #[case::bundled_all_verbose(&["pg", "show", "-av", "one.xmp"], false, true, true)]
    #[case::options_after_target(&["pg", "show", "one.xmp", "--all", "-v"], false, true, true)]
    fn parses_show_display_options(
        #[case] args: &[&str],
        #[case] basic: bool,
        #[case] all: bool,
        #[case] verbose_expected: bool,
    ) {
        let Command::Show {
            display, verbose, ..
        } = parse(args)
        else {
            panic!("expected show command");
        };
        assert_eq!(display.basic, basic);
        assert_eq!(display.all, all);
        assert_eq!(display.key, None);
        assert_eq!(verbose, verbose_expected);
    }

    #[rstest]
    #[case::single_dash(&["pg", "set", "photo", "-starts-with-dash"], "-starts-with-dash")]
    #[case::double_dash(&["pg", "set", "photo", "--verbose"], "--verbose")]
    #[case::empty(&["pg", "set", "photo", ""], "")]
    #[case::after_key(&["pg", "set", "-k", "XMP-dc:Creator", "photo", "-text"], "-text")]
    fn parses_set_value(#[case] args: &[&str], #[case] expected: &str) {
        let Command::Set { target, value, .. } = parse(args) else {
            panic!("expected set command");
        };
        assert_eq!(target, "photo");
        assert_eq!(value, expected);
    }

    #[test]
    fn parses_show_key_with_verbose() {
        let Command::Show {
            display, verbose, ..
        } = parse(&[
            "pg",
            "show",
            "--key",
            "XMP-dc:Creator",
            "--verbose",
            "one.xmp",
        ])
        else {
            panic!("expected show command");
        };
        assert_eq!(display.key.as_deref(), Some("XMP-dc:Creator"));
        assert!(verbose);
    }

    #[test]
    fn parses_export_force_after_targets() {
        let Command::Export { force, targets } = parse(&["pg", "export", "one.jpg", "--force"])
        else {
            panic!("expected export command");
        };
        assert!(force);
        assert_eq!(targets, ["one.jpg"]);
    }

    #[rstest]
    #[case::show_conflicting_modes(&["pg", "show", "--all", "--basic", "one.xmp"], ErrorKind::ArgumentConflict)]
    #[case::show_conflicting_modes_verbose(&["pg", "show", "--basic", "--all", "--verbose", "one.xmp"], ErrorKind::ArgumentConflict)]
    #[case::show_basic_with_key(&["pg", "show", "--basic", "--key", "XMP-dc:Creator", "one.xmp"], ErrorKind::ArgumentConflict)]
    #[case::show_all_with_key(&["pg", "show", "--all", "--key", "XMP-dc:Creator", "one.xmp"], ErrorKind::ArgumentConflict)]
    #[case::show_short_conflicting_modes(&["pg", "show", "-a", "-b", "one.xmp"], ErrorKind::ArgumentConflict)]
    #[case::show_short_key_with_all(&["pg", "show", "-k", "XMP-dc:Creator", "-a", "one.xmp"], ErrorKind::ArgumentConflict)]
    #[case::show_bundled_conflicting_modes(&["pg", "show", "-ab", "one.xmp"], ErrorKind::ArgumentConflict)]
    #[case::set_extra_argument(&["pg", "set", "a", "b", "c"], ErrorKind::UnknownArgument)]
    #[case::clear_extra_argument(&["pg", "clear", "a", "b"], ErrorKind::UnknownArgument)]
    #[case::edit_extra_argument(&["pg", "edit", "a", "b"], ErrorKind::UnknownArgument)]
    #[case::no_subcommand(&["pg"], ErrorKind::DisplayHelpOnMissingArgumentOrSubcommand)]
    #[case::unknown_subcommand(&["pg", "foo"], ErrorKind::InvalidSubcommand)]
    #[case::unknown_option(&["pg", "show", "--nope", "one.xmp"], ErrorKind::UnknownArgument)]
    #[case::export_unknown_short_option(&["pg", "export", "-x", "one.jpg"], ErrorKind::UnknownArgument)]
    #[case::edit_rejects_key(&["pg", "edit", "--key", "XMP-dc:Creator", "photo"], ErrorKind::UnknownArgument)]
    #[case::key_without_value(&["pg", "show", "one.xmp", "--key"], ErrorKind::InvalidValue)]
    #[case::show_without_target(&["pg", "show"], ErrorKind::MissingRequiredArgument)]
    #[case::export_without_target(&["pg", "export"], ErrorKind::MissingRequiredArgument)]
    #[case::set_without_args(&["pg", "set"], ErrorKind::MissingRequiredArgument)]
    #[case::set_without_value(&["pg", "set", "photo"], ErrorKind::MissingRequiredArgument)]
    #[case::clear_without_target(&["pg", "clear"], ErrorKind::MissingRequiredArgument)]
    #[case::edit_without_target(&["pg", "edit"], ErrorKind::MissingRequiredArgument)]
    fn rejects_invalid_arguments(#[case] args: &[&str], #[case] expected: ErrorKind) {
        let error = Cli::try_parse_from(args).err().unwrap();
        assert_eq!(error.kind(), expected);
    }

    #[rstest]
    #[case::help(&["pg", "--help"], ErrorKind::DisplayHelp)]
    #[case::help_short(&["pg", "-h"], ErrorKind::DisplayHelp)]
    #[case::export_help(&["pg", "export", "--help"], ErrorKind::DisplayHelp)]
    #[case::show_help(&["pg", "show", "-h"], ErrorKind::DisplayHelp)]
    #[case::set_help(&["pg", "set", "--help"], ErrorKind::DisplayHelp)]
    #[case::clear_help(&["pg", "clear", "--help"], ErrorKind::DisplayHelp)]
    #[case::edit_help(&["pg", "edit", "--help"], ErrorKind::DisplayHelp)]
    #[case::version(&["pg", "--version"], ErrorKind::DisplayVersion)]
    #[case::version_short(&["pg", "-V"], ErrorKind::DisplayVersion)]
    fn short_circuits_on_help_and_version(#[case] args: &[&str], #[case] expected: ErrorKind) {
        let error = Cli::try_parse_from(args).err().unwrap();
        assert_eq!(error.kind(), expected);
    }
}
