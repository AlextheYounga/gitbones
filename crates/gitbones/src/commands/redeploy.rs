use std::path::Path;

use anyhow::Result;
use console::style;

use crate::config;
use crate::ssh;

const BONES_TOML: &str = ".bones/bones.toml";

pub async fn run() -> Result<()> {
    let bones_toml = Path::new(BONES_TOML);
    let cfg = config::load(bones_toml)?;

    let git_dir = &cfg.data.git_dir;

    println!(
        "Redeploying {} on {}...",
        style(&cfg.data.project_name).cyan().bold(),
        style(&cfg.data.host).cyan()
    );

    let session = ssh::connect(&cfg).await?;

    // Run pre-receive (doctor + pre-deploy)
    println!("Running pre-receive...");
    ssh::stream_cmd(
        &session,
        &format!("GIT_DIR={git_dir} {git_dir}/hooks/pre-receive </dev/null"),
    )
    .await?;

    // Run post-receive (checkout + deploy + post-deploy)
    println!("Running post-receive...");
    ssh::stream_cmd(
        &session,
        &format!("GIT_DIR={git_dir} {git_dir}/hooks/post-receive </dev/null"),
    )
    .await?;

    session.close().await?;

    println!("\n{} Redeployment complete.", style("Done!").green().bold());

    Ok(())
}
