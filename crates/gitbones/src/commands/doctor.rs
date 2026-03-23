use std::fs;
use std::path::Path;
use std::process::Command;

use anyhow::Result;
use console::style;

use crate::config;
use crate::ssh;

const BONES_DIR: &str = ".bones";
const BONES_TOML: &str = ".bones/bones.toml";

pub async fn run(local_only: bool) -> Result<()> {
    println!("{}", style("gitbones doctor").bold());

    let mut issues: Vec<String> = Vec::new();

    check_bones_structure(&mut issues);
    check_deployment_naming(&mut issues);
    check_pre_push_symlink(&mut issues);

    if !local_only {
        let bones_toml = Path::new(BONES_TOML);
        match config::load(bones_toml) {
            Ok(cfg) => check_remote(&cfg, &mut issues).await,
            Err(e) => issues.push(format!("Cannot load config: {e}")),
        }
    }

    if issues.is_empty() {
        println!("\n{} All checks passed.", style("OK").green().bold());
        Ok(())
    } else {
        println!();
        for issue in &issues {
            println!("  {} {issue}", style("!").red().bold());
        }
        anyhow::bail!(
            "Doctor found {} issue{}",
            issues.len(),
            if issues.len() == 1 { "" } else { "s" }
        );
    }
}

fn check_bones_structure(issues: &mut Vec<String>) {
    let bones_dir = Path::new(BONES_DIR);

    if !bones_dir.exists() {
        issues.push(".bones/ directory does not exist".into());
        return;
    }

    let expected = [".bones/bones.toml", ".bones/hooks", ".bones/deployment"];

    for path in &expected {
        if !Path::new(path).exists() {
            issues.push(format!("{path} is missing"));
        }
    }
}

fn check_deployment_naming(issues: &mut Vec<String>) {
    let deployment_dir = Path::new(".bones/deployment");
    if !deployment_dir.exists() {
        return;
    }

    let Ok(entries) = fs::read_dir(deployment_dir) else {
        return;
    };

    for entry in entries {
        let Ok(entry) = entry else { continue };
        let name = entry.file_name();
        let name = name.to_string_lossy();

        // Scripts must start with a numeric prefix like "01_"
        let has_numeric_prefix = name.chars().take_while(char::is_ascii_digit).count() > 0;

        if !has_numeric_prefix {
            issues.push(format!(
                "Deployment script '{name}' does not start with a numeric prefix (e.g. 01_)"
            ));
        }
    }
}

fn check_pre_push_symlink(issues: &mut Vec<String>) {
    let link = Path::new(".git/hooks/pre-push");

    if !link.symlink_metadata().is_ok_and(|m| m.is_symlink()) {
        issues.push(".git/hooks/pre-push is not symlinked".into());
        return;
    }

    let Ok(target) = fs::read_link(link) else {
        issues.push(".git/hooks/pre-push: cannot read symlink target".into());
        return;
    };

    let expected = Path::new("../../.bones/hooks/pre-push");
    if target != expected {
        issues.push(format!(
            ".git/hooks/pre-push points to '{}', expected '{}'",
            target.display(),
            expected.display()
        ));
    }
}

async fn check_remote(cfg: &config::BonesConfig, issues: &mut Vec<String>) {
    let session = match ssh::connect(cfg).await {
        Ok(s) => s,
        Err(e) => {
            issues.push(format!("Cannot connect to remote: {e}"));
            return;
        }
    };

    let git_dir = &cfg.data.git_dir;

    // Check gitbones-remote is globally available
    if ssh::run_cmd(&session, "command -v gitbones-remote")
        .await
        .is_err()
    {
        issues.push("gitbones-remote is not available on the remote".into());
    }

    // Check bones/ folder exists on remote
    let check_bones = format!("test -d {git_dir}/bones");
    if ssh::run_cmd(&session, &check_bones).await.is_err() {
        issues.push(format!(
            "{git_dir}/bones/ does not exist on remote (run 'gitbones push')"
        ));
    }

    // Check local .bones/ is in sync with remote
    check_rsync_sync(cfg, issues);

    // Check hooks are symlinked properly
    let check_hooks = format!(
        "for hook in {git_dir}/bones/hooks/*; do \
            name=$(basename \"$hook\"); \
            link=\"{git_dir}/hooks/$name\"; \
            if [ ! -L \"$link\" ] || [ \"$(readlink \"$link\")\" != \"$hook\" ]; then \
                echo \"$name\"; \
            fi; \
         done"
    );
    match ssh::run_cmd(&session, &check_hooks).await {
        Ok(output) => {
            for hook in output.lines() {
                let hook = hook.trim();
                if !hook.is_empty() {
                    issues.push(format!(
                        "{git_dir}/hooks/{hook} is not properly symlinked to bones/hooks/{hook}"
                    ));
                }
            }
        }
        Err(e) => issues.push(format!("Failed to check remote hook symlinks: {e}")),
    }

    let _ = session.close().await;
}

fn check_rsync_sync(cfg: &config::BonesConfig, issues: &mut Vec<String>) {
    let user = &cfg.permissions.defaults.deploy;
    let host = &cfg.data.host;
    let port = &cfg.data.port;
    let git_dir = &cfg.data.git_dir;
    let dest = format!("{user}@{host}:{git_dir}/bones/");

    let output = Command::new("rsync")
        .args([
            "-avnc",
            "--delete",
            "-e",
            &format!("ssh -p {port}"),
            &format!("{BONES_DIR}/"),
            &dest,
        ])
        .output();

    let output = match output {
        Ok(o) => o,
        Err(e) => {
            issues.push(format!("Failed to run rsync sync check: {e}"));
            return;
        }
    };

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        issues.push(format!("rsync sync check failed: {stderr}"));
        return;
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    let changed: Vec<&str> = stdout
        .lines()
        .filter(|line| {
            let line = line.trim();
            // Skip rsync summary/header lines and directory-only entries
            !line.is_empty()
                && !line.starts_with("sending ")
                && !line.starts_with("sent ")
                && !line.starts_with("total ")
                && !line.ends_with('/')
        })
        .collect();

    if !changed.is_empty() {
        issues.push(format!(
            "Local .bones/ is out of sync with remote (run 'gitbones push'). Changed files:\n{}",
            changed
                .iter()
                .map(|f| format!("      {f}"))
                .collect::<Vec<_>>()
                .join("\n")
        ));
    }
}
