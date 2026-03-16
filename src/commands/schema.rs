use anyhow::Result;
use colored::Colorize;

use crate::config::{write_schema, SCHEMA_FILE};

pub fn sync() -> Result<()> {
    let dir = std::path::Path::new(".");
    let schema_path = dir.join(SCHEMA_FILE);

    let was_current = schema_path
        .metadata()
        .is_ok_and(|_| {
            std::fs::read_to_string(&schema_path)
                .map(|existing| existing == crate::config::SCHEMA_JSON)
                .unwrap_or(false)
        });

    write_schema(dir)?;

    if was_current {
        println!(
            "{} {} is already up to date.",
            "info:".cyan().bold(),
            SCHEMA_FILE
        );
    } else {
        println!("{} {}", "updated:".green().bold(), SCHEMA_FILE);
    }

    Ok(())
}
