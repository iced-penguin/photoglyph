mod exiftool;
mod filesystem;
mod glob;
mod port;
mod usecase;

use clap::{Args, Parser, Subcommand};
use port::SidecarStore;
use std::path::PathBuf;
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
    let exiftool = exiftool::ExifTool;
    let filesystem = filesystem::LocalFileSystem;
    if let Err(error) = exiftool.ensure_available() {
        eprintln!("pg: {error}");
        return ExitCode::from(1);
    }

    let mut failed = false;
    match cli.command {
        Command::Export { force, targets } => {
            for image in expand_all(&targets) {
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
            for target in expand_all(&targets) {
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
                    Ok(text) if text.is_empty() => {}
                    Ok(text) => println!("{text}"),
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
            usecase::edit(target.as_ref(), &filesystem),
            &mut failed,
        ),
    }
    if failed {
        ExitCode::from(2)
    } else {
        ExitCode::SUCCESS
    }
}

fn expand_all(patterns: &[String]) -> Vec<PathBuf> {
    patterns
        .iter()
        .flat_map(|pattern| glob::expand(pattern))
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
