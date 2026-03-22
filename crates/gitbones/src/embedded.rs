use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::path::Path;

use anyhow::{Context, Result};
use rust_embed::Embed;

#[derive(Embed)]
#[folder = "../../kit/"]
struct Kit;

pub fn scaffold(bones_dir: &Path) -> Result<()> {
    for file_path in Kit::iter() {
        let Some(asset) = Kit::get(&file_path) else {
            continue;
        };
        let dest = bones_dir.join(file_path.as_ref());

        if let Some(parent) = dest.parent() {
            fs::create_dir_all(parent)
                .with_context(|| format!("Failed to create {}", parent.display()))?;
        }

        fs::write(&dest, asset.data.as_ref())
            .with_context(|| format!("Failed to write {}", dest.display()))?;

        // Make hook scripts executable
        if file_path.starts_with("hooks/") {
            fs::set_permissions(&dest, fs::Permissions::from_mode(0o755))
                .with_context(|| format!("Failed to set permissions on {}", dest.display()))?;
        }
    }
    Ok(())
}
