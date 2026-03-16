use anyhow::Result;
use colored::Colorize;
use std::time::SystemTime;

use crate::config::Config;

pub fn clear() -> Result<()> {
    let (config, _) = Config::find()?;
    let cache_dir = config.cache.cache_dir();

    if !cache_dir.exists() {
        println!("{} Cache directory does not exist — nothing to clear.", "info:".cyan().bold());
        return Ok(());
    }

    let mut cleared = 0u32;
    for entry in std::fs::read_dir(&cache_dir)?.flatten() {
        if entry.path().extension().map(|x| x == "ok").unwrap_or(false) {
            std::fs::remove_file(entry.path())?;
            cleared += 1;
        }
    }

    if cleared == 0 {
        println!("{} No cache entries found.", "info:".cyan().bold());
    } else {
        println!("{} Cleared {} cache entr{}.", "done:".green().bold(), cleared, if cleared == 1 { "y" } else { "ies" });
    }

    Ok(())
}

pub fn list() -> Result<()> {
    let (config, _) = Config::find()?;
    let cache_dir = config.cache.cache_dir();

    if !cache_dir.exists() || !config.cache.enabled {
        println!(
            "{} Caching is {} Cache directory: {}",
            "info:".cyan().bold(),
            if config.cache.enabled { "enabled." } else { "disabled." },
            cache_dir.display()
        );
        return Ok(());
    }

    let mut entries: Vec<(String, u64)> = std::fs::read_dir(&cache_dir)?
        .flatten()
        .filter(|e| e.path().extension().map(|x| x == "ok").unwrap_or(false))
        .filter_map(|e| {
            let key = e.path().file_stem()?.to_string_lossy().to_string();
            let age_secs = e
                .metadata()
                .ok()
                .and_then(|m| m.modified().ok())
                .and_then(|t| SystemTime::now().duration_since(t).ok())
                .map(|d| d.as_secs())
                .unwrap_or(0);
            Some((key, age_secs))
        })
        .collect();

    if entries.is_empty() {
        println!("{} No cache entries.", "info:".cyan().bold());
        return Ok(());
    }

    entries.sort_by(|a, b| a.1.cmp(&b.1));

    println!(
        "{:<64}  {}",
        "Key (SHA-256)".dimmed(),
        "Age".dimmed()
    );
    println!("{}", "─".repeat(74).dimmed());

    for (key, age_secs) in &entries {
        let age = format_age(*age_secs);
        println!("{:<64}  {}", key, age.dimmed());
    }

    println!();
    println!("{} {} entr{}.", entries.len().to_string().cyan().bold(), "total".dimmed(), if entries.len() == 1 { "y" } else { "ies" });

    Ok(())
}

fn format_age(secs: u64) -> String {
    if secs < 60 {
        format!("{}s ago", secs)
    } else if secs < 3600 {
        format!("{}m ago", secs / 60)
    } else if secs < 86400 {
        format!("{}h ago", secs / 3600)
    } else {
        format!("{}d ago", secs / 86400)
    }
}
