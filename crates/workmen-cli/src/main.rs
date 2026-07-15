use std::path::PathBuf;

use clap::{Parser, Subcommand};
use workmen_core::WorkmenError;
use workmen_core::project::{ProjectInitializer, ProjectRoot};
use workmen_core::report::{ExitCode, Report, ReportFormat};
use workmen_core::scan::{ScanMode, ScanRequest, scan_project};

/// Workmen: game asset workbench.
#[derive(Parser)]
#[command(name = "workmen", version, about = "Workmen: game asset workbench", long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Read-only scan of a game project
    Scan {
        #[arg(value_name = "PATH")]
        path: PathBuf,
        /// Output format: text (default) or json
        #[arg(long, default_value_t = ReportFormat::Text)]
        format: ReportFormat,
    },
    /// Validate assets against resolved Profiles
    Validate {
        #[arg(value_name = "PATH")]
        path: PathBuf,
        /// Output format: text (default) or json
        #[arg(long, default_value_t = ReportFormat::Text)]
        format: ReportFormat,
    },
    /// Initialize a .workmen/ project contract directory
    Init {
        #[arg(value_name = "PATH")]
        path: PathBuf,
        #[arg(long)]
        confirm: bool,
    },
}

fn main() {
    let cli = Cli::parse();
    // The dispatch functions consume the format and the path by
    // value, so this match is the only place we need `cli.command`.
    // The dispatch functions return `(Report, ReportFormat)` so
    // we don't have to re-match on `&cli.command` after the move.
    let (report, format) = match cli.command {
        Command::Scan { path, format } => run_scan(&path, format),
        Command::Validate { path, format } => run_validate(&path, format),
        Command::Init { path, confirm } => match run_init(&path, confirm) {
            Ok(()) => (
                Report::Scan(workmen_core::report::ScanReport {
                    root: PathBuf::new(),
                    files: vec![],
                    diagnostics: vec![],
                }),
                ReportFormat::Text,
            ),
            Err(e) => {
                eprintln!("{e}");
                std::process::exit(ExitCode::ConfigFailure as i32);
            }
        },
    };
    print!("{}", report.render(format));
    std::process::exit(report.exit_code() as i32);
}

/// `workmen scan <path>` -- read-only scan that surfaces files and
/// diagnostics. The plan's exit-code contract: 0 on clean scan,
/// 1 if any diagnostics, 2 on config failure.
fn run_scan(path: &std::path::Path, format: ReportFormat) -> (Report, ReportFormat) {
    let result: Result<Report, WorkmenError> = (|| {
        let root = ProjectRoot::discover(path)?;
        let scan = scan_project(ScanRequest {
            root: &root,
            config: None,
            mode: ScanMode::ReadOnly,
        })?;
        Ok(Report::Scan(workmen_core::report::ScanReport {
            root: root.path().to_path_buf(),
            files: scan.files,
            diagnostics: scan.diagnostics,
        }))
    })();
    match result {
        Ok(r) => (r, format),
        Err(e) => {
            eprintln!("{e}");
            std::process::exit(ExitCode::ConfigFailure as i32);
        }
    }
}

/// `workmen validate <path>` -- placeholder. The full validate
/// flow (resolve profile, run rules) is a future task. For now
/// we synthesize a validation report with no issues so the
/// exit-code path is exercised.
fn run_validate(path: &std::path::Path, format: ReportFormat) -> (Report, ReportFormat) {
    let result: Result<Report, WorkmenError> = (|| {
        let _root = ProjectRoot::discover(path)?;
        Ok(Report::Validation(workmen_core::report::ValidationReport {
            root: path.to_path_buf(),
            profile_id: workmen_core::model::ProfileId("default".to_string()),
            platform: workmen_core::model::Platform::Web,
            issues: vec![],
        }))
    })();
    match result {
        Ok(r) => (r, format),
        Err(e) => {
            eprintln!("{e}");
            std::process::exit(ExitCode::ConfigFailure as i32);
        }
    }
}

/// `workmen init <path>` with `--confirm`.
///
/// Without `--confirm`, prints the proposed `.workmen/project.yaml` and
/// `.workmen/specs/` paths and exits 2 (the design's "config failure"
/// exit code). With `--confirm`, writes the files atomically and
/// returns.
fn run_init(path: &std::path::Path, confirm: bool) -> Result<(), WorkmenError> {
    let root = ProjectRoot::discover(path)?;
    let preview = ProjectInitializer::preview(&root)?;
    if !confirm {
        eprintln!("init preview for: {}", root.path().display());
        eprintln!("  will create: {}", preview.project_yaml_path.display());
        eprintln!("  will create: {}", preview.specs_dir.display());
        eprintln!();
        eprintln!("re-run with --confirm to apply");
        std::process::exit(ExitCode::ConfigFailure as i32);
    }
    ProjectInitializer::commit(preview, true)?;
    Ok(())
}