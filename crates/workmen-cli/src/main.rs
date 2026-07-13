use std::path::PathBuf;

use clap::{Parser, Subcommand};
use workmen_core::WorkmenError;

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
        Command::Init {
            path: _,
            confirm: _,
        } => Err(WorkmenError::internal("init: not implemented yet")),
    };
    if let Err(e) = result {
        eprintln!("{e}");
        std::process::exit(2);
    }
}
