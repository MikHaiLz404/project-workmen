use std::path::PathBuf;

use clap::{Parser, Subcommand};
use workmen_core::WorkmenError;
use workmen_core::project::{ProjectInitializer, ProjectRoot};

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
    },
    /// Validate assets against resolved Profiles
    Validate {
        #[arg(value_name = "PATH")]
        path: PathBuf,
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
    let result: Result<(), WorkmenError> = match cli.command {
        Command::Scan { path: _ } => Err(WorkmenError::internal("scan: not implemented yet")),
        Command::Validate { path: _ } => {
            Err(WorkmenError::internal("validate: not implemented yet"))
        }
        Command::Init { path, confirm } => run_init(&path, confirm),
    };
    if let Err(e) = result {
        eprintln!("{e}");
        std::process::exit(2);
    }
}

/// `workmen init <path>` with `--confirm`.
///
/// Without `--confirm`, prints the proposed `.workmen/project.yaml` and
/// `.workmen/specs/` paths and exits 2 (the design's "config failure"
/// exit code). With `--confirm`, writes the files atomically and
/// returns. The error path uses `WorkmenError::Display` via `main()`.
fn run_init(path: &std::path::Path, confirm: bool) -> Result<(), WorkmenError> {
    let root = ProjectRoot::discover(path)?;
    let preview = ProjectInitializer::preview(&root)?;
    if !confirm {
        eprintln!("init preview for: {}", root.path().display());
        eprintln!("  will create: {}", preview.project_yaml_path.display());
        eprintln!("  will create: {}", preview.specs_dir.display());
        eprintln!();
        eprintln!("re-run with --confirm to apply");
        std::process::exit(2);
    }
    ProjectInitializer::commit(preview, true)?;
    Ok(())
}
