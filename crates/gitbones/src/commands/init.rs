use std::env;
use std::fs;
use std::os::unix::fs as unix_fs;
use std::path::Path;

use anyhow::{Context, Result};
use console::style;
use inquire::Confirm;

use crate::config;
use crate::embedded;
use crate::git;
use crate::prompts;
use crate::ssh;

const BONES_DIR: &str = ".bones";
const BONES_TOML: &str = ".bones/bones.toml";

pub async fn run() -> Result<()> {
    println!(
        "{}\n\n\
         This will:\n  \
         1. Create a .bones/ folder with hooks and deployment scripts\n  \
         2. Collect project configuration (remote, host, permissions)\n  \
         3. Update .gitignore\n  \
         4. Symlink the pre-push hook into .git/hooks/\n  \
         5. Create a bare repo on the remote (if needed)\n  \
         6. Upload the post-receive hook to the remote\n\n\
         A git remote URL must already be configured for the deployment remote.\n",
        style("gitbones init").bold()
    );

    let proceed = Confirm::new("Continue?")
        .with_default(true)
        .prompt()?;
    if !proceed {
        println!("Aborted.");
        return Ok(());
    }

    let repo = git::open_repo()?;

    // Extract scaffold to .bones/
    let bones_dir = Path::new(BONES_DIR);
    if bones_dir.exists() {
        println!(".bones/ already exists, skipping scaffold extraction.");
    } else {
        println!("Creating .bones/ scaffold...");
        embedded::scaffold(bones_dir)?;
    }

    // Update .gitignore
    update_gitignore()?;

    let bones_toml = Path::new(BONES_TOML);
    let cfg = load_or_collect_config(bones_toml)?;

    // Validate the remote exists
    git::validate_remote_exists(&repo, &cfg.data.remote_name)?;

    // Save config
    config::save(&cfg, bones_toml)?;
    println!("Saved config to .bones/bones.toml");

    // Symlink pre-push hook
    symlink_pre_push()?;

    // Remote setup over SSH
    println!("\nConnecting to remote...");
    let session = ssh::connect(&cfg).await?;

    ssh::create_bare_repo(&session, &cfg.data.git_dir).await?;

    let post_receive = embedded::read_asset("hooks/post-receive")?;
    ssh::upload_post_receive(&session, &cfg.data.git_dir, &post_receive).await?;

    session.close().await?;

    println!(
        "\n{} Run {} to sync .bones/ to the remote.",
        style("Done!").green().bold(),
        style("gitbones push").cyan()
    );

    Ok(())
}

fn load_or_collect_config(bones_toml: &Path) -> Result<config::BonesConfig> {
    if bones_toml.exists() {
        let existing = config::load(bones_toml)?;
        if config::is_configured(&existing) {
            println!("Loading existing config from .bones/bones.toml...");
            return Ok(existing);
        }
        println!("Config is incomplete, running prompts...");
    }
    let project_name = repo_directory_name()?;
    prompts::collect(&project_name)
}

fn update_gitignore() -> Result<()> {
    let gitignore = Path::new(".gitignore");
    let entry = ".bones";

    if gitignore.exists() {
        let content = fs::read_to_string(gitignore)?;
        if content.lines().any(|line| line.trim() == entry) {
            return Ok(());
        }
        let separator = if content.ends_with('\n') { "" } else { "\n" };
        fs::write(gitignore, format!("{content}{separator}{entry}\n"))?;
    } else {
        fs::write(gitignore, format!("{entry}\n"))?;
    }

    println!("Added .bones to .gitignore");
    Ok(())
}

fn symlink_pre_push() -> Result<()> {
    let hooks_dir = Path::new(".git/hooks");
    fs::create_dir_all(hooks_dir)?;

    let link = hooks_dir.join("pre-push");
    let target = Path::new("../../.bones/hooks/pre-push");

    if link.exists() || link.symlink_metadata().is_ok() {
        fs::remove_file(&link)
            .with_context(|| format!("Failed to remove existing {}", link.display()))?;
    }

    unix_fs::symlink(target, &link)
        .with_context(|| format!("Failed to symlink {}", link.display()))?;

    println!("Symlinked .git/hooks/pre-push -> .bones/hooks/pre-push");
    Ok(())
}

fn repo_directory_name() -> Result<String> {
    let cwd = env::current_dir()?;
    let name = cwd
        .file_name()
        .map_or_else(|| "project".into(), |n| n.to_string_lossy().to_string());
    Ok(name)
}
