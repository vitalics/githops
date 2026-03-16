use anyhow::Result;
use colored::Colorize;

use crate::config::{write_schema, Config};
use crate::git::hooks_dir;

pub use githops_core::sync_hooks::{sync_to_hooks, GITHOPS_MARKER};

pub fn run(force: bool) -> Result<()> {
    let (config, config_path) = Config::find()?;
    let hooks_dir = hooks_dir()?;

    // Keep the schema file up-to-date so every team member has it after sync.
    let project_dir = config_path.parent().unwrap_or(std::path::Path::new("."));
    write_schema(project_dir)?;

    let (installed, skipped) = sync_to_hooks(&config, &hooks_dir, force)?;

    println!();
    println!(
        "Synced {} hook(s) from {}{}.",
        installed.to_string().cyan().bold(),
        config_path.display().to_string().cyan(),
        if skipped > 0 {
            format!(" ({} skipped — pre-existing unmanaged hooks)", skipped)
        } else {
            String::new()
        }
    );

    Ok(())
}

