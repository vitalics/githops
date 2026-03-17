use anyhow::Result;
use colored::Colorize;

use crate::config::{write_schema, Config};
use crate::git::hooks_dir;
use crate::logger::LogLayer;

pub use githops_core::sync_hooks::{sync_to_hooks, GITHOPS_MARKER};

pub fn run(force: bool) -> Result<()> {
    crate::log_verbose!(LogLayer::YamlResolve, "loading config");
    let (config, config_path) = Config::find()?;
    crate::log_info!(LogLayer::YamlResolve, "config loaded from {}", config_path.display());

    let hooks_dir = hooks_dir()?;
    crate::log_verbose!(LogLayer::YamlExec, "hooks dir: {}", hooks_dir.display());

    // Keep the schema file up-to-date so every team member has it after sync.
    let project_dir = config_path.parent().unwrap_or(std::path::Path::new("."));
    crate::log_verbose!(LogLayer::SchemaValidation, "updating schema");
    write_schema(project_dir)?;
    crate::log_trace!(LogLayer::SchemaValidation, "schema written to {}", project_dir.join(".githops/githops.schema.json").display());

    crate::log_verbose!(LogLayer::YamlExec, "syncing hooks (force={})", force);
    let (installed, skipped) = sync_to_hooks(&config, &hooks_dir, force)?;
    crate::log_info!(LogLayer::YamlExec, "installed {} hook(s), skipped {}", installed, skipped);

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

