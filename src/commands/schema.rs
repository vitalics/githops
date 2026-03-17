use anyhow::Result;
use colored::Colorize;

use crate::config::{write_schema, SCHEMA_FILE};
use crate::logger::LogLayer;

pub fn sync() -> Result<()> {
    let dir = std::path::Path::new(".");
    let schema_path = dir.join(SCHEMA_FILE);

    crate::log_verbose!(LogLayer::SchemaValidation, "checking schema at {}", schema_path.display());

    let was_current = schema_path
        .metadata()
        .is_ok_and(|_| {
            std::fs::read_to_string(&schema_path)
                .map(|existing| existing == crate::config::SCHEMA_JSON)
                .unwrap_or(false)
        });

    write_schema(dir)?;

    if was_current {
        crate::log_info!(LogLayer::SchemaValidation, "schema is already up to date");
        println!(
            "{} {} is already up to date.",
            "info:".cyan().bold(),
            SCHEMA_FILE
        );
    } else {
        crate::log_info!(LogLayer::SchemaValidation, "schema updated: {}", schema_path.display());
        println!("{} {}", "updated:".green().bold(), SCHEMA_FILE);
    }

    Ok(())
}
