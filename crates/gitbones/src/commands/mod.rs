mod doctor;
mod init;
mod push;
mod redeploy;
mod version;

use anyhow::Result;
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
    /// Re-run the deployment hooks without pushing
    Redeploy,
    /// Print the version
    Version,
}

pub async fn run(cli: &Cli) -> Result<()> {
    match &cli.command {
        Command::Init => init::run().await,
        Command::Doctor { local } => doctor::run(*local).await,
        Command::Push => push::run().await,
        Command::Redeploy => redeploy::run().await,
        Command::Version => {
            version::run();
            Ok(())
        }
    }
}
