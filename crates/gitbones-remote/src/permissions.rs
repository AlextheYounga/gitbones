use std::os::unix::fs::PermissionsExt;
use std::path::Path;
use std::process::Command;
use std::{fs, io};

use anyhow::{Context, Result, bail};
use walkdir::WalkDir;

use crate::config::BonesConfig;

pub fn chown_to_deploy_user(cfg: &BonesConfig) -> Result<()> {
    let user = &cfg.permissions.defaults.deploy;
    let worktree = &cfg.data.worktree;

    if !Path::new(worktree).exists() {
        fs::create_dir_all(worktree)
            .with_context(|| format!("Failed to create worktree directory: {worktree}"))?;
        println!("Created worktree directory: {worktree}");
    }

    run_chown(&format!("{user}:{user}"), worktree, true)?;
    println!("Changed ownership of {worktree} to {user}");
    Ok(())
}

pub fn harden(cfg: &BonesConfig) -> Result<()> {
    let defaults = &cfg.permissions.defaults;
    let worktree = &cfg.data.worktree;

    // Apply default ownership
    let ownership = format!("{}:{}", defaults.owner, defaults.group);
    run_chown(&ownership, worktree, true)?;
    println!("Set ownership of {worktree} to {ownership}");

    // Apply default dir_mode and file_mode
    let dir_mode = parse_mode(&defaults.dir_mode)?;
    let file_mode = parse_mode(&defaults.file_mode)?;
    apply_default_modes(worktree, dir_mode, file_mode)?;
    println!(
        "Applied default modes: dirs={}, files={}",
        defaults.dir_mode, defaults.file_mode
    );

    // Apply path overrides
    for override_entry in &cfg.permissions.paths {
        let target = Path::new(worktree).join(&override_entry.path);
        if !target.exists() {
            println!(
                "Warning: override path '{}' does not exist, skipping",
                target.display()
            );
            continue;
        }

        let mode = parse_mode(&override_entry.mode)?;

        if override_entry.recursive {
            apply_recursive_mode(&target, mode)?;
        } else if let Some(ref path_type) = override_entry.path_type {
            match path_type.as_str() {
                "dir" | "file" => apply_single_mode(&target, mode)?,
                other => bail!("Unknown path type: {other}"),
            }
        } else {
            apply_single_mode(&target, mode)?;
        }

        println!(
            "Applied mode {} to {}{}",
            override_entry.mode,
            override_entry.path,
            if override_entry.recursive {
                " (recursive)"
            } else {
                ""
            }
        );
    }

    Ok(())
}

fn run_chown(ownership: &str, path: &str, recursive: bool) -> Result<()> {
    let mut cmd = Command::new("chown");
    if recursive {
        cmd.arg("-R");
    }
    cmd.arg(ownership).arg(path);

    let status = cmd
        .status()
        .with_context(|| format!("Failed to chown {path}"))?;

    if !status.success() {
        bail!("chown {ownership} {path} failed");
    }
    Ok(())
}


fn parse_mode(mode_str: &str) -> Result<u32> {
    u32::from_str_radix(mode_str, 8).with_context(|| format!("Invalid mode: {mode_str}"))
}

fn apply_default_modes(worktree: &str, dir_mode: u32, file_mode: u32) -> Result<()> {
    for entry in WalkDir::new(worktree) {
        let entry = entry.with_context(|| format!("Failed to walk {worktree}"))?;
        // Follow symlinks so a symlink to a directory gets dir_mode, not file_mode
        let metadata = fs::metadata(entry.path())
            .with_context(|| format!("Failed to read metadata for {}", entry.path().display()))?;

        let mode = if metadata.is_dir() {
            dir_mode
        } else {
            file_mode
        };
        set_permissions(entry.path(), mode)?;
    }
    Ok(())
}

fn apply_recursive_mode(path: &Path, mode: u32) -> Result<()> {
    for entry in WalkDir::new(path) {
        let entry = entry.with_context(|| format!("Failed to walk {}", path.display()))?;
        set_permissions(entry.path(), mode)?;
    }
    Ok(())
}

fn apply_single_mode(path: &Path, mode: u32) -> Result<()> {
    set_permissions(path, mode)
}

fn set_permissions(path: &Path, mode: u32) -> Result<()> {
    fs::set_permissions(path, fs::Permissions::from_mode(mode)).map_err(|e| {
        io::Error::new(
            e.kind(),
            format!("chmod {:o} {}: {e}", mode, path.display()),
        )
    })?;
    Ok(())
}
