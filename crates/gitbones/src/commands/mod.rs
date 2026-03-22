mod init;
mod version;

use anyhow::{Result, bail};
use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(name = "gitbones", about = "Git deployment scaffolding tool")]
pub struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Set up gitbones in the current repository
    Init,
    /// Check local and remote environment health
    Doctor {
        /// Skip remote checks
        #[arg(long)]
        local: bool,
    },
    /// Sync .bones/ folder to the remote bare repo
    Push,
    /// Print the version
    Version,
}

pub fn run(cli: &Cli) -> Result<()> {
    match &cli.command {
        Command::Init => init::run(),
        Command::Doctor { .. } => bail!("doctor is not yet implemented"),
        Command::Push => bail!("push is not yet implemented"),
        Command::Version => {
            version::run();
            Ok(())
        }
    }
}
