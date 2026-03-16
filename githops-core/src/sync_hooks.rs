use anyhow::Result;
use colored::Colorize;
use std::path::Path;

use crate::config::Config;
use crate::hooks::ALL_HOOKS;

/// Marker written into every hook script so we can identify githops-managed files.
pub const GITHOPS_MARKER: &str = "# GITHOPS_MANAGED";

/// Write/remove hook scripts in `hooks_dir` to match `config`.
///
/// When `force` is `true`, pre-existing hooks that are not managed by githops
/// are overwritten instead of skipped.
///
/// Returns `(installed_count, skipped_count)`.
pub fn sync_to_hooks(config: &Config, hooks_dir: &Path, force: bool) -> Result<(usize, usize)> {
    std::fs::create_dir_all(hooks_dir)?;

    let mut installed = 0usize;
    let mut skipped = 0usize;

    for hook_info in ALL_HOOKS {
        let hook_cfg = match config.hooks.get(hook_info.name) {
            Some(cfg) => cfg,
            None => continue,
        };

        let resolved = hook_cfg.resolved_commands(&config.definitions);
        let active_count = resolved.iter().filter(|c| !c.test).count();

        if !hook_cfg.enabled || active_count == 0 {
            continue;
        }

        let hook_path = hooks_dir.join(hook_info.name);
        let script = build_hook_script(hook_info.name, active_count);

        if hook_path.exists() {
            let existing = std::fs::read_to_string(&hook_path).unwrap_or_default();
            if !existing.contains(GITHOPS_MARKER) {
                if !force {
                    println!(
                        "{} {} — not managed by githops, skipping (use {} to overwrite)",
                        "skip:".yellow().bold(),
                        hook_info.name,
                        "githops sync --force".cyan()
                    );
                    skipped += 1;
                    continue;
                }
                println!(
                    "{} {} — overwriting unmanaged hook",
                    "force:".yellow().bold(),
                    hook_info.name
                );
            }
        }

        std::fs::write(&hook_path, &script)?;
        make_executable(&hook_path)?;
        println!("{} {}", "synced:".green().bold(), hook_info.name);
        installed += 1;
    }

    // Remove hooks that were managed by githops but are no longer in config.
    for hook_info in ALL_HOOKS {
        let hook_path = hooks_dir.join(hook_info.name);
        if !hook_path.exists() {
            continue;
        }
        let existing = std::fs::read_to_string(&hook_path).unwrap_or_default();
        if !existing.contains(GITHOPS_MARKER) {
            continue;
        }
        let configured = config
            .hooks
            .get(hook_info.name)
            .map(|c| {
                let resolved = c.resolved_commands(&config.definitions);
                c.enabled && resolved.iter().any(|cmd| !cmd.test)
            })
            .unwrap_or(false);
        if !configured {
            std::fs::remove_file(&hook_path)?;
            println!("{} {}", "removed:".dimmed(), hook_info.name);
        }
    }

    Ok((installed, skipped))
}

fn build_hook_script(hook_name: &str, command_count: usize) -> String {
    format!(
        r#"#!/bin/sh
{marker}
# Hook: {name}
# Managed by githops — do not edit manually.
# Run `githops sync` to regenerate.
# Commands configured: {count}

exec githops check {name} "$@"
"#,
        marker = GITHOPS_MARKER,
        name = hook_name,
        count = command_count
    )
}

#[cfg(unix)]
fn make_executable(path: &Path) -> Result<()> {
    use std::os::unix::fs::PermissionsExt;
    let mut perms = std::fs::metadata(path)?.permissions();
    perms.set_mode(perms.mode() | 0o111);
    std::fs::set_permissions(path, perms)?;
    Ok(())
}

#[cfg(not(unix))]
fn make_executable(_path: &Path) -> Result<()> {
    Ok(())
}
