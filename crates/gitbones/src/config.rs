use std::fs;
use std::path::Path;

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct BonesConfig {
    pub data: Data,
    #[serde(default)]
    pub permissions: Permissions,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Data {
    #[serde(default)]
    pub remote_name: String,
    #[serde(default)]
    pub project_name: String,
    #[serde(default)]
    pub host: String,
    #[serde(default = "default_port")]
    pub port: String,
    #[serde(default)]
    pub git_dir: String,
    #[serde(default)]
    pub worktree: String,
    #[serde(default = "default_branch")]
    pub branch: String,
}

#[derive(Debug, Default, Serialize, Deserialize)]
pub struct Permissions {
    #[serde(default)]
    pub defaults: PermissionDefaults,
    #[serde(default)]
    pub paths: Vec<PathOverride>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct PermissionDefaults {
    #[serde(default = "default_deploy")]
    pub deploy: String,
    #[serde(default = "default_owner")]
    pub owner: String,
    #[serde(default = "default_group")]
    pub group: String,
    #[serde(default = "default_dir_mode")]
    pub dir_mode: String,
    #[serde(default = "default_file_mode")]
    pub file_mode: String,
}

impl Default for PermissionDefaults {
    fn default() -> Self {
        Self {
            deploy: default_deploy(),
            owner: default_owner(),
            group: default_group(),
            dir_mode: default_dir_mode(),
            file_mode: default_file_mode(),
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct PathOverride {
    pub path: String,
    pub mode: String,
    #[serde(default)]
    pub recursive: bool,
    #[serde(rename = "type", skip_serializing_if = "Option::is_none")]
    pub path_type: Option<String>,
}

fn default_port() -> String { "22".into() }
fn default_branch() -> String { "master".into() }
fn default_deploy() -> String { "git".into() }
fn default_owner() -> String { "applications".into() }
fn default_group() -> String { "www-data".into() }
fn default_dir_mode() -> String { "750".into() }
fn default_file_mode() -> String { "640".into() }

pub fn is_configured(config: &BonesConfig) -> bool {
    let d = &config.data;
    !d.remote_name.is_empty()
        && !d.project_name.is_empty()
        && !d.host.is_empty()
        && !d.git_dir.is_empty()
        && !d.worktree.is_empty()
}

pub fn load(path: &Path) -> Result<BonesConfig> {
    let content = fs::read_to_string(path)
        .with_context(|| format!("Failed to read {}", path.display()))?;
    let config: BonesConfig = toml::from_str(&content)
        .with_context(|| format!("Failed to parse {}", path.display()))?;
    Ok(config)
}

pub fn save(config: &BonesConfig, path: &Path) -> Result<()> {
    let content = toml::to_string_pretty(config)
        .context("Failed to serialize config")?;
    fs::write(path, content)
        .with_context(|| format!("Failed to write {}", path.display()))?;
    Ok(())
}
