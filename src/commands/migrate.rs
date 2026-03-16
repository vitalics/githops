use anyhow::{bail, Result};
use colored::Colorize;
use std::collections::BTreeMap;
use std::path::Path;

use crate::cli::MigrateSource;
use crate::commands::sync::GITHOPS_MARKER;
use crate::config::{write_schema, Command, CommandEntry, Config, HookConfig, CONFIG_FILE, SCHEMA_FILE};
use crate::git::hooks_dir;
use crate::hooks::ALL_HOOKS;

const MIGRATION_DIR: &str = ".githops/migration";

pub fn run(source: MigrateSource) -> Result<()> {
    let (config, tool) = match source {
        MigrateSource::Husky => (collect_husky_config()?, "husky"),
        MigrateSource::Lefthook => (collect_lefthook_config()?, "lefthook"),
    };

    write_config(&config)?;
    write_revert_script(tool)?;
    remove_old_hooks(&config, tool)?;

    println!();
    println!("Running {}...", "githops sync".cyan().bold());
    println!();
    crate::commands::sync::run(false)?;

    Ok(())
}

// ---------------------------------------------------------------------------
// Husky
// ---------------------------------------------------------------------------

fn collect_husky_config() -> Result<Config> {
    let husky_dir = Path::new(".husky");
    if !husky_dir.exists() {
        bail!("No .husky/ directory found. Are you in the right repository?");
    }

    let mut config = Config::default();

    for hook_info in ALL_HOOKS {
        let hook_path = husky_dir.join(hook_info.name);
        if !hook_path.exists() {
            continue;
        }

        let content = std::fs::read_to_string(&hook_path)?;
        let commands = extract_husky_commands(&content);

        if commands.is_empty() {
            continue;
        }

        println!(
            "{} {} ({} command(s))",
            "found:".green().bold(),
            hook_info.name,
            commands.len()
        );

        config.hooks.set(
            hook_info.name,
            HookConfig { enabled: true, parallel: false, commands },
        );
    }

    Ok(config)
}

/// Extract meaningful shell commands from a husky hook script, skipping
/// shebangs, blank lines, and husky's own boilerplate.
fn extract_husky_commands(script: &str) -> Vec<CommandEntry> {
    script
        .lines()
        .filter(|line| {
            let t = line.trim();
            !t.is_empty()
                && !t.starts_with('#')
                && !t.starts_with("#!/")
                && t != "set -e"
                && !t.contains(". \"$(dirname -- \"$0\")/_/husky.sh\"")
                && !t.contains(". \"$(dirname \"$0\")/_/husky.sh\"")
                && !t.starts_with(". \"")
        })
        .enumerate()
        .map(|(i, line)| {
            CommandEntry::Inline(Command {
                name: format!("step-{}", i + 1),
                run: line.trim().to_string(),
                depends: vec![],
                env: BTreeMap::new(),
                test: false,
                cache: None,
            })
        })
        .collect()
}

// ---------------------------------------------------------------------------
// Lefthook
// ---------------------------------------------------------------------------

fn collect_lefthook_config() -> Result<Config> {
    let candidates = ["lefthook.yml", "lefthook.yaml"];
    let lefthook_path = candidates
        .iter()
        .map(Path::new)
        .find(|p| p.exists())
        .ok_or_else(|| anyhow::anyhow!("No lefthook.yml / lefthook.yaml found."))?;

    let content = std::fs::read_to_string(lefthook_path)?;
    let raw: serde_yaml::Value = serde_yaml::from_str(&content)?;

    let mut config = Config::default();

    let table = match &raw {
        serde_yaml::Value::Mapping(m) => m,
        _ => bail!("Invalid lefthook.yml format"),
    };

    for (key, value) in table {
        let hook_name = match key.as_str() {
            Some(n) => n,
            None => continue,
        };

        if ALL_HOOKS.iter().all(|h| h.name != hook_name) {
            continue;
        }

        let commands = extract_lefthook_commands(hook_name, value);
        if commands.is_empty() {
            continue;
        }

        println!(
            "{} {} ({} command(s))",
            "found:".green().bold(),
            hook_name,
            commands.len()
        );

        config.hooks.set(
            hook_name,
            HookConfig { enabled: true, parallel: false, commands },
        );
    }

    Ok(config)
}

fn extract_lefthook_commands(hook_name: &str, value: &serde_yaml::Value) -> Vec<CommandEntry> {
    let mut commands = Vec::new();

    let mapping = match value.as_mapping() {
        Some(m) => m,
        None => return commands,
    };

    if let Some(cmds) = mapping.get("commands").and_then(|v| v.as_mapping()) {
        for (name, cmd_value) in cmds {
            let name_str = name.as_str().unwrap_or("step").to_string();
            let run = cmd_value
                .as_mapping()
                .and_then(|m| m.get("run"))
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string();

            if run.is_empty() {
                continue;
            }

            let mut env = BTreeMap::new();
            if let Some(env_map) = cmd_value
                .as_mapping()
                .and_then(|m| m.get("env"))
                .and_then(|v| v.as_mapping())
            {
                for (k, v) in env_map {
                    if let (Some(k), Some(v)) = (k.as_str(), v.as_str()) {
                        env.insert(k.to_string(), v.to_string());
                    }
                }
            }

            commands.push(CommandEntry::Inline(Command {
                name: name_str,
                run,
                depends: vec![],
                env,
                test: false,
                cache: None,
            }));
        }
    }

    if mapping.get("scripts").is_some() {
        eprintln!(
            "{} {}: 'scripts' block is not yet supported by migrate, add them manually.",
            "warn:".yellow().bold(),
            hook_name
        );
    }

    commands
}

// ---------------------------------------------------------------------------
// Shared: write githops.yaml + schema
// ---------------------------------------------------------------------------

fn write_config(config: &Config) -> Result<()> {
    let config_path = Path::new(CONFIG_FILE);
    if config_path.exists() {
        bail!(
            "{} already exists. Remove it or back it up before migrating.",
            CONFIG_FILE
        );
    }

    write_schema(Path::new("."))?;
    println!("{} {}", "created:".green().bold(), SCHEMA_FILE);

    let yaml_body = serde_yaml::to_string(config)?;
    let content = format!(
        "# yaml-language-server: $schema={}\n# Migrated by `githops migrate`\n{}",
        SCHEMA_FILE, yaml_body
    );
    std::fs::write(config_path, &content)?;
    println!("{} {}", "created:".green().bold(), CONFIG_FILE);

    Ok(())
}

/// Remove hooks installed by the old tool so that `githops sync` can replace them.
/// Only removes files that are not already managed by githops and that look like
/// they were written by the given tool.
fn remove_old_hooks(config: &Config, tool: &str) -> Result<()> {
    let dir = hooks_dir()?;

    for hook_info in ALL_HOOKS {
        if config.hooks.get(hook_info.name).is_none() {
            continue;
        }

        let path = dir.join(hook_info.name);
        if !path.exists() {
            continue;
        }

        let content = std::fs::read_to_string(&path).unwrap_or_default();

        // Never touch files already managed by githops.
        if content.contains(GITHOPS_MARKER) {
            continue;
        }

        let is_old_tool = match tool {
            "husky" => {
                content.contains("husky.sh")
                    || content.contains("husky run")
                    || content.contains("# husky")
            }
            "lefthook" => {
                content.contains("lefthook run")
                    || content.contains("lefthook-runner")
                    || content.contains("# lefthook")
            }
            _ => false,
        };

        if is_old_tool {
            std::fs::remove_file(&path)?;
            println!(
                "{} {} (was managed by {})",
                "removed:".yellow().bold(),
                hook_info.name,
                tool
            );
        }
    }

    Ok(())
}

// ---------------------------------------------------------------------------
// Revert script
// ---------------------------------------------------------------------------

fn write_revert_script(tool: &str) -> Result<()> {
    let dir = Path::new(MIGRATION_DIR).join(tool);
    std::fs::create_dir_all(&dir)?;

    let script_path = dir.join("revert.sh");
    let content = build_revert_script(tool);
    std::fs::write(&script_path, &content)?;
    make_executable(&script_path)?;

    println!(
        "{} {}",
        "created:".green().bold(),
        script_path.display()
    );
    println!(
        "  {} To undo this migration later, run: {}",
        "tip:".dimmed(),
        script_path.display().to_string().cyan()
    );

    Ok(())
}

fn build_revert_script(tool: &str) -> String {
    let (tool_note, reinstall_hint) = match tool {
        "husky" => (
            "Your .husky/ directory was not modified.",
            "To re-activate husky, run:  npx husky install",
        ),
        "lefthook" => (
            "Your lefthook.yml / lefthook.yaml was not modified.",
            "To re-activate lefthook, run:  lefthook install",
        ),
        _ => ("", ""),
    };

    format!(
        r#"#!/usr/bin/env bash
# Generated by `githops migrate --from {tool}`
# Run this script to undo the migration and restore {tool}.
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/../../.." && pwd)"
cd "$REPO_ROOT"

echo "Reverting githops migration ({tool})..."
echo ""

# Remove githops-managed hooks from .git/hooks/
if command -v githops &>/dev/null; then
    githops destroy
else
    echo "warn: githops not in PATH — remove managed hooks manually from .git/hooks/"
fi

# Remove githops config files
rm -f {config} {schema}
echo "removed: {config} {schema}"

echo ""
echo "{tool_note}"
echo "{reinstall_hint}"
"#,
        tool = tool,
        config = CONFIG_FILE,
        schema = SCHEMA_FILE,
        tool_note = tool_note,
        reinstall_hint = reinstall_hint,
    )
}

#[cfg(unix)]
fn make_executable(path: &Path) -> Result<()> {
    use std::os::unix::fs::PermissionsExt;
    let mut perms = std::fs::metadata(path)?.permissions();
    perms.set_mode(perms.mode() | 0o755);
    std::fs::set_permissions(path, perms)?;
    Ok(())
}

#[cfg(not(unix))]
fn make_executable(_path: &Path) -> Result<()> {
    Ok(())
}
