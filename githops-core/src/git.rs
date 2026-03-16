use anyhow::{Context, Result};
use std::path::PathBuf;
use std::process::Command;

/// Returns the path of the `.git` directory for the current working directory.
pub fn git_dir() -> Result<PathBuf> {
    let output = Command::new("git")
        .args(["rev-parse", "--git-dir"])
        .output()
        .context("Failed to run git rev-parse --git-dir")?;

    if !output.status.success() {
        anyhow::bail!(
            "Not inside a git repository. Run `git init` first.\n{}",
            String::from_utf8_lossy(&output.stderr).trim()
        );
    }

    let path = String::from_utf8_lossy(&output.stdout).trim().to_string();
    Ok(PathBuf::from(path))
}

/// Returns the hooks directory, respecting `core.hooksPath` if configured.
pub fn hooks_dir() -> Result<PathBuf> {
    let output = Command::new("git")
        .args(["config", "core.hooksPath"])
        .output()
        .context("Failed to query git config")?;

    if output.status.success() {
        let path = String::from_utf8_lossy(&output.stdout).trim().to_string();
        if !path.is_empty() {
            return Ok(PathBuf::from(path));
        }
    }

    Ok(git_dir()?.join("hooks"))
}
