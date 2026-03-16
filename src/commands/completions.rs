use anyhow::{Context, Result};
use clap_complete::Shell;
use colored::Colorize;
use std::io::Write as _;
use std::path::PathBuf;

// ── paths ──────────────────────────────────────────────────────────────────

fn home() -> PathBuf {
    dirs::home_dir().unwrap_or_else(|| PathBuf::from("~"))
}

fn completion_path(shell: Shell) -> PathBuf {
    let h = home();
    match shell {
        Shell::Bash => h.join(".local/share/bash-completion/completions/githops"),
        Shell::Zsh => h.join(".zfunc/_githops"),
        Shell::Fish => h.join(".config/fish/completions/githops.fish"),
        Shell::Elvish => h.join(".config/elvish/lib/completions/githops.elv"),
        Shell::PowerShell => h.join("Documents/PowerShell/Completions/githops.ps1"),
        _ => h.join(format!(".githops/completions/githops.{shell}")),
    }
}

fn rc_path(shell: Shell) -> Option<PathBuf> {
    let h = home();
    match shell {
        Shell::Bash => Some(h.join(".bashrc")),
        Shell::Zsh => Some(h.join(".zshrc")),
        _ => None,
    }
}

// ── rc patching ────────────────────────────────────────────────────────────

const RC_MARKER: &str = "# githops completions";

/// Lines to append to the shell rc so completions are loaded automatically.
fn rc_snippet(shell: Shell) -> Option<String> {
    match shell {
        Shell::Bash => Some(format!(
            "{marker}\nsource {path}\n",
            marker = RC_MARKER,
            path = completion_path(shell).display(),
        )),
        Shell::Zsh => Some(format!(
            "{marker}\nfpath=(~/.zfunc $fpath)\nautoload -Uz compinit && compinit\n",
            marker = RC_MARKER,
        )),
        _ => None,
    }
}

/// Append `snippet` to `rc` only if the marker is not already present.
/// Returns true if the file was modified.
fn patch_rc(rc: &std::path::Path, snippet: &str) -> Result<bool> {
    let existing = std::fs::read_to_string(rc).unwrap_or_default();
    if existing.contains(RC_MARKER) {
        return Ok(false);
    }
    let mut file = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(rc)
        .with_context(|| format!("opening {}", rc.display()))?;
    writeln!(file, "\n{snippet}")?;
    Ok(true)
}

/// Remove the block starting with `RC_MARKER` from an rc file.
/// Returns true if anything was removed.
fn unpatch_rc(rc: &std::path::Path) -> Result<bool> {
    let Ok(content) = std::fs::read_to_string(rc) else {
        return Ok(false);
    };
    if !content.contains(RC_MARKER) {
        return Ok(false);
    }
    // Drop lines from the marker up to (but not including) the next blank line
    // that follows the block, keeping everything else intact.
    let filtered: Vec<&str> = {
        let mut skip = false;
        content
            .lines()
            .filter(|line| {
                if line.trim() == RC_MARKER.trim() {
                    skip = true;
                }
                if skip && line.trim().is_empty() {
                    skip = false;
                    return false; // drop the trailing blank line too
                }
                !skip
            })
            .collect()
    };
    std::fs::write(rc, filtered.join("\n") + "\n")?;
    Ok(true)
}

// ── shell detection ────────────────────────────────────────────────────────

fn detect_shell() -> Result<Shell> {
    let shell_path = std::env::var("SHELL").context(
        "$SHELL is not set — use `githops completions print <shell>` for manual installation",
    )?;
    let name = std::path::Path::new(&shell_path)
        .file_name()
        .and_then(|s| s.to_str())
        .unwrap_or("");
    match name {
        "bash" => Ok(Shell::Bash),
        "zsh" => Ok(Shell::Zsh),
        "fish" => Ok(Shell::Fish),
        "elvish" => Ok(Shell::Elvish),
        other => anyhow::bail!(
            "Unsupported shell '{other}'. Use `githops completions print <shell>` for manual installation."
        ),
    }
}

// ── completion file writer ─────────────────────────────────────────────────

fn write_completion(shell: Shell) -> Result<PathBuf> {
    let path = completion_path(shell);
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)
            .with_context(|| format!("creating {}", parent.display()))?;
    }
    let mut buf = Vec::new();
    clap_complete::generate(
        shell,
        &mut <crate::cli::Cli as clap::CommandFactory>::command(),
        "githops",
        &mut buf,
    );
    std::fs::write(&path, &buf).with_context(|| format!("writing {}", path.display()))?;
    Ok(path)
}

// ── public commands ────────────────────────────────────────────────────────

pub fn init() -> Result<()> {
    let shell = detect_shell()?;

    // 1. Write the completion script
    let path = write_completion(shell)?;
    println!(
        "{} {} ({})",
        "installed:".green().bold(),
        path.display(),
        format!("{shell}").cyan()
    );

    // 2. Patch the shell rc so it's loaded automatically
    if let (Some(rc), Some(snippet)) = (rc_path(shell), rc_snippet(shell)) {
        match patch_rc(&rc, &snippet) {
            Ok(true) => println!(
                "{} added completion loader to {}",
                "updated:".green().bold(),
                rc.display()
            ),
            Ok(false) => println!(
                "{} {} already configured",
                "info:".cyan().bold(),
                rc.display()
            ),
            Err(e) => println!("{} could not update rc: {e}", "warn:".yellow().bold()),
        }
        println!(
            "{} {}",
            "note:".yellow().bold(),
            "Restart your shell or open a new terminal to activate completions."
        );
    } else {
        // Fish and others — no rc patching needed or supported
        println!(
            "{} Completions active in new {} sessions.",
            "note:".yellow().bold(),
            format!("{shell}").cyan()
        );
    }

    Ok(())
}

pub fn remove() -> Result<()> {
    let shells = [
        Shell::Bash,
        Shell::Zsh,
        Shell::Fish,
        Shell::Elvish,
        Shell::PowerShell,
    ];
    let mut removed = false;

    for shell in shells {
        // Remove completion file
        let path = completion_path(shell);
        if path.exists() {
            std::fs::remove_file(&path)
                .with_context(|| format!("removing {}", path.display()))?;
            println!("{} {}", "removed:".green().bold(), path.display());
            removed = true;
        }

        // Remove rc block
        if let Some(rc) = rc_path(shell) {
            match unpatch_rc(&rc) {
                Ok(true) => {
                    println!(
                        "{} removed completion block from {}",
                        "updated:".green().bold(),
                        rc.display()
                    );
                }
                Ok(false) => {}
                Err(e) => println!("{} could not update rc: {e}", "warn:".yellow().bold()),
            }
        }
    }

    if !removed {
        println!("{} No completion files found.", "info:".cyan().bold());
    }

    Ok(())
}
