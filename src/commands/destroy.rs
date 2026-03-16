use anyhow::Result;
use colored::Colorize;

use crate::git::hooks_dir;
use crate::hooks::ALL_HOOKS;

use super::sync::GITHOPS_MARKER;

pub fn run() -> Result<()> {
    let hooks_dir = hooks_dir()?;
    let mut removed = 0usize;

    for hook_info in ALL_HOOKS {
        let hook_path = hooks_dir.join(hook_info.name);
        if !hook_path.exists() {
            continue;
        }

        let content = std::fs::read_to_string(&hook_path).unwrap_or_default();
        if !content.contains(GITHOPS_MARKER) {
            continue;
        }

        std::fs::remove_file(&hook_path)?;
        println!("{} {}", "removed:".red().bold(), hook_info.name);
        removed += 1;
    }

    if removed == 0 {
        println!("{}", "No githops-managed hooks found.".dimmed());
    } else {
        println!();
        println!(
            "Removed {} githops-managed hook(s) from {}.",
            removed.to_string().cyan().bold(),
            hooks_dir.display().to_string().cyan()
        );
    }

    Ok(())
}
