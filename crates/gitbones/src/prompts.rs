use anyhow::Result;
use inquire::Text;

use crate::config::{BonesConfig, Data, PermissionDefaults, Permissions};

pub fn collect(project_name_hint: &str) -> Result<BonesConfig> {
    let remote_name = Text::new("Remote name:")
        .with_default("production")
        .with_help_message("e.g. production, staging")
        .prompt()?;

    let project_name = Text::new("Project name:")
        .with_default(project_name_hint)
        .prompt()?;

    let host = Text::new("Host:")
        .with_help_message("e.g. deploy.example.com")
        .prompt()?;

    let port = Text::new("Port:")
        .with_default("22")
        .prompt()?;

    let default_git_dir = format!("/home/git/{project_name}.git");
    let git_dir = Text::new("Git directory (bare repo path on remote):")
        .with_default(&default_git_dir)
        .prompt()?;

    let default_worktree = format!("/var/www/{project_name}");
    let worktree = Text::new("Worktree path on remote:")
        .with_default(&default_worktree)
        .prompt()?;

    let branch = Text::new("Branch:")
        .with_default("master")
        .prompt()?;

    let deploy_user = Text::new("Deploy user (SSH user):")
        .with_default("git")
        .prompt()?;

    let service_user = Text::new("Service user (final file owner):")
        .with_default("applications")
        .prompt()?;

    let service_group = Text::new("Service group:")
        .with_default("www-data")
        .prompt()?;

    let dir_mode = Text::new("Default directory mode:")
        .with_default("750")
        .prompt()?;

    let file_mode = Text::new("Default file mode:")
        .with_default("640")
        .prompt()?;

    Ok(BonesConfig {
        data: Data {
            remote_name,
            project_name,
            host,
            port,
            git_dir,
            worktree,
            branch,
        },
        permissions: Permissions {
            defaults: PermissionDefaults {
                deploy: deploy_user,
                owner: service_user,
                group: service_group,
                dir_mode,
                file_mode,
            },
            paths: vec![],
        },
    })
}
