use anyhow::Result;
use colored::Colorize;
use std::path::Path;

use crate::config::{write_schema, Command, CommandEntry, Config, HookConfig, SCHEMA_FILE, CONFIG_FILE};

pub fn run() -> Result<()> {
    let config_path = Path::new(CONFIG_FILE);

    if config_path.exists() {
        println!(
            "{} {} already exists.",
            "warn:".yellow().bold(),
            CONFIG_FILE
        );
        // Still refresh the schema in case the binary was updated.
        write_schema(Path::new("."))?;
        println!("{} {}", "updated:".green().bold(), SCHEMA_FILE);
        return Ok(());
    }

    write_schema(Path::new("."))?;
    println!("{} {}", "created:".green().bold(), SCHEMA_FILE);

    // Build example config
    let mut config = Config::default();

    config.hooks.pre_commit = Some(HookConfig {
        enabled: true,
        parallel: false,
        commands: vec![
            CommandEntry::Inline(Command {
                name: "fmt".to_string(),
                run: "echo \"Run your formatter here, e.g. cargo fmt --check\"".to_string(),
                depends: vec![],
                env: Default::default(),
                test: false,
                cache: None,
            }),
            CommandEntry::Inline(Command {
                name: "lint".to_string(),
                run: "echo \"Run your linter here, e.g. cargo clippy\"".to_string(),
                depends: vec![],
                env: Default::default(),
                test: false,
                cache: None,
            }),
        ],
    });

    config.hooks.commit_msg = Some(HookConfig {
        enabled: true,
        parallel: false,
        commands: vec![CommandEntry::Inline(Command {
            name: "validate".to_string(),
            // $1 is the path to the file containing the commit message
            run: "echo \"Validate commit message in $1\"".to_string(),
            depends: vec![],
            env: Default::default(),
            test: false,
            cache: None,
        })],
    });

    config.hooks.pre_push = Some(HookConfig {
        enabled: true,
        parallel: false,
        commands: vec![CommandEntry::Inline(Command {
            name: "test".to_string(),
            run: "echo \"Run your test suite here, e.g. cargo test\"".to_string(),
            depends: vec![],
            env: Default::default(),
            test: false,
            cache: None,
        })],
    });

    // Serialize with yaml-language-server directive prepended
    let yaml_body = serde_yaml::to_string(&config)?;
    let content = format!(
        "# yaml-language-server: $schema={}\n{}",
        SCHEMA_FILE, yaml_body
    );
    std::fs::write(config_path, &content)?;
    println!("{} {}", "created:".green().bold(), CONFIG_FILE);

    // Install the example hooks immediately so the repo is ready to use.
    println!();
    super::sync::run(false)?;

    println!();
    println!(
        "Edit {} to configure your hooks, then run {} to apply changes.",
        CONFIG_FILE.cyan(),
        "githops sync".cyan().bold()
    );

    Ok(())
}
