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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::{Command, CommandEntry, Config, HookConfig};
    use std::collections::BTreeMap;
    use tempfile::TempDir;

    #[test]
    fn test_build_hook_script_contains_marker() {
        let script = build_hook_script("pre-commit", 3);
        assert!(script.contains("GITHOPS_MANAGED"));
    }

    #[test]
    fn test_build_hook_script_contains_hook_name() {
        let script = build_hook_script("commit-msg", 1);
        assert!(script.contains("commit-msg"));
    }

    #[test]
    fn test_build_hook_script_has_shebang() {
        let script = build_hook_script("pre-push", 2);
        assert!(script.starts_with("#!/"));
    }

    #[test]
    fn test_build_hook_script_calls_githops_check() {
        let script = build_hook_script("pre-commit", 1);
        assert!(script.contains("githops check"));
    }

    #[test]
    fn test_sync_creates_hook_files() {
        let dir = TempDir::new().unwrap();
        let mut config = Config::default();
        config.hooks.pre_commit = Some(HookConfig {
            enabled: true,
            parallel: false,
            commands: vec![CommandEntry::Inline(Command {
                name: "lint".into(),
                run: "echo lint".into(),
                depends: vec![],
                env: BTreeMap::new(),
                test: false,
                cache: None,
            })],
        });

        let (installed, skipped) = sync_to_hooks(&config, dir.path(), false).unwrap();
        assert_eq!(installed, 1);
        assert_eq!(skipped, 0);
        assert!(dir.path().join("pre-commit").exists());
    }

    #[test]
    fn test_sync_hook_script_content_is_correct() {
        let dir = TempDir::new().unwrap();
        let mut config = Config::default();
        config.hooks.pre_commit = Some(HookConfig {
            enabled: true,
            parallel: false,
            commands: vec![CommandEntry::Inline(Command {
                name: "lint".into(),
                run: "echo lint".into(),
                depends: vec![],
                env: BTreeMap::new(),
                test: false,
                cache: None,
            })],
        });

        sync_to_hooks(&config, dir.path(), false).unwrap();
        let content = std::fs::read_to_string(dir.path().join("pre-commit")).unwrap();
        assert!(content.contains("GITHOPS_MANAGED"));
        assert!(content.contains("pre-commit"));
    }

    #[test]
    fn test_sync_skips_disabled_hook() {
        let dir = TempDir::new().unwrap();
        let mut config = Config::default();
        config.hooks.pre_commit = Some(HookConfig {
            enabled: false, // disabled
            parallel: false,
            commands: vec![CommandEntry::Inline(Command {
                name: "lint".into(),
                run: "echo lint".into(),
                depends: vec![],
                env: BTreeMap::new(),
                test: false,
                cache: None,
            })],
        });

        let (installed, _skipped) = sync_to_hooks(&config, dir.path(), false).unwrap();
        assert_eq!(installed, 0);
        assert!(!dir.path().join("pre-commit").exists());
    }

    #[test]
    fn test_sync_skips_test_only_commands() {
        let dir = TempDir::new().unwrap();
        let mut config = Config::default();
        config.hooks.pre_commit = Some(HookConfig {
            enabled: true,
            parallel: false,
            commands: vec![CommandEntry::Inline(Command {
                name: "lint".into(),
                run: "echo lint".into(),
                depends: vec![],
                env: BTreeMap::new(),
                test: true, // test-only, should not create hook
                cache: None,
            })],
        });

        let (installed, _) = sync_to_hooks(&config, dir.path(), false).unwrap();
        assert_eq!(installed, 0);
    }

    #[test]
    fn test_sync_does_not_overwrite_unmanaged_hook() {
        let dir = TempDir::new().unwrap();
        // Write an unmanaged hook (no GITHOPS_MANAGED marker)
        std::fs::write(dir.path().join("pre-commit"), "#!/bin/sh\necho manual").unwrap();

        let mut config = Config::default();
        config.hooks.pre_commit = Some(HookConfig {
            enabled: true,
            parallel: false,
            commands: vec![CommandEntry::Inline(Command {
                name: "lint".into(),
                run: "echo lint".into(),
                depends: vec![],
                env: BTreeMap::new(),
                test: false,
                cache: None,
            })],
        });

        let (_installed, skipped) = sync_to_hooks(&config, dir.path(), false).unwrap();
        assert_eq!(skipped, 1);
        // Original content preserved
        let content = std::fs::read_to_string(dir.path().join("pre-commit")).unwrap();
        assert!(content.contains("manual"));
    }

    #[test]
    fn test_sync_force_overwrites_unmanaged_hook() {
        let dir = TempDir::new().unwrap();
        std::fs::write(dir.path().join("pre-commit"), "#!/bin/sh\necho manual").unwrap();

        let mut config = Config::default();
        config.hooks.pre_commit = Some(HookConfig {
            enabled: true,
            parallel: false,
            commands: vec![CommandEntry::Inline(Command {
                name: "lint".into(),
                run: "echo lint".into(),
                depends: vec![],
                env: BTreeMap::new(),
                test: false,
                cache: None,
            })],
        });

        let (installed, _) = sync_to_hooks(&config, dir.path(), true).unwrap();
        assert_eq!(installed, 1);
        let content = std::fs::read_to_string(dir.path().join("pre-commit")).unwrap();
        assert!(content.contains("GITHOPS_MANAGED"));
    }

    #[test]
    fn test_sync_removes_obsolete_managed_hook() {
        let dir = TempDir::new().unwrap();
        // Write a managed hook that is no longer configured
        let managed_content =
            "#!/bin/sh\n# GITHOPS_MANAGED\nexec githops check pre-commit \"$@\"\n";
        std::fs::write(dir.path().join("pre-commit"), managed_content).unwrap();

        // Config has no pre-commit hook
        let config = Config::default();

        sync_to_hooks(&config, dir.path(), false).unwrap();
        // Managed hook should be removed
        assert!(!dir.path().join("pre-commit").exists());
    }

    #[test]
    fn test_sync_is_idempotent() {
        let dir = TempDir::new().unwrap();
        let mut config = Config::default();
        config.hooks.pre_commit = Some(HookConfig {
            enabled: true,
            parallel: false,
            commands: vec![CommandEntry::Inline(Command {
                name: "lint".into(),
                run: "echo lint".into(),
                depends: vec![],
                env: BTreeMap::new(),
                test: false,
                cache: None,
            })],
        });

        sync_to_hooks(&config, dir.path(), false).unwrap();
        let content1 = std::fs::read_to_string(dir.path().join("pre-commit")).unwrap();
        sync_to_hooks(&config, dir.path(), false).unwrap();
        let content2 = std::fs::read_to_string(dir.path().join("pre-commit")).unwrap();
        assert_eq!(content1, content2);
    }
}
