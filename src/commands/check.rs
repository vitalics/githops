use anyhow::{bail, Result};
use colored::Colorize;
use std::collections::HashMap;
use std::path::PathBuf;
use std::process::{Command as Cmd, Stdio};
use std::sync::Mutex;

use crate::config::{Command, Config};
use crate::hooks::find_hook;
use githops_core::cache;

pub use githops_core::config::validate_depends_pub;

// ---------------------------------------------------------------------------
// Entry point
// ---------------------------------------------------------------------------

/// Shared cache context threaded through execution helpers.
struct CacheCtx {
    dir: PathBuf,
    root: PathBuf,
}

pub fn run(hook_name: &str, args: &[String]) -> Result<()> {
    if find_hook(hook_name).is_none() {
        bail!(
            "Unknown hook: '{}'. See `githops graph` for all supported hooks.",
            hook_name
        );
    }

    let (config, _) = Config::find()?;

    let hook_cfg = match config.hooks.get(hook_name) {
        Some(cfg) => cfg,
        None => return Ok(()),
    };

    let commands = hook_cfg.resolved_commands(&config.definitions);

    if !hook_cfg.enabled || commands.is_empty() {
        return Ok(());
    }

    // Filter out test-only commands during normal hook execution.
    let commands: Vec<_> = commands.into_iter().filter(|c| !c.test).collect();

    validate_depends_pub(&commands)?;

    let waves = build_execution_waves(&commands)?;

    let cache_ctx = if config.cache.enabled {
        let root = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
        Some(CacheCtx { dir: config.cache.cache_dir(), root })
    } else {
        None
    };

    println!(
        "{} {} hook · {} command(s){}{}",
        "githops:".dimmed(),
        hook_name.bold(),
        commands.len(),
        if hook_cfg.parallel { " · parallel" } else { "" },
        if cache_ctx.is_some() { " · cache on" } else { "" },
    );

    for wave in &waves {
        if hook_cfg.parallel && wave.len() > 1 {
            run_wave_parallel(wave, args, cache_ctx.as_ref())?;
        } else {
            for cmd in wave {
                run_one_cached(cmd, args, /*capture=*/ false, cache_ctx.as_ref())?;
            }
        }
    }

    Ok(())
}

// ---------------------------------------------------------------------------
// Topological sort — Kahn's algorithm
// Returns commands grouped into "waves". All commands in a wave have no
// unresolved dependencies, so they can run concurrently.
// ---------------------------------------------------------------------------

fn build_execution_waves<'a>(commands: &'a [Command]) -> Result<Vec<Vec<&'a Command>>> {
    let n = commands.len();
    let name_to_idx: HashMap<&str, usize> = commands
        .iter()
        .enumerate()
        .map(|(i, c)| (c.name.as_str(), i))
        .collect();

    // in_degree[i]    = number of not-yet-completed deps of command i
    // dependents[i]   = indices of commands that list i in their `depends`
    let mut in_degree = vec![0usize; n];
    let mut dependents: Vec<Vec<usize>> = vec![vec![]; n];

    for (i, cmd) in commands.iter().enumerate() {
        for dep in &cmd.depends {
            let j = name_to_idx[dep.as_str()];
            in_degree[i] += 1;
            dependents[j].push(i);
        }
    }

    let mut ready: Vec<usize> = (0..n).filter(|&i| in_degree[i] == 0).collect();
    let mut waves: Vec<Vec<&Command>> = Vec::new();
    let mut processed = 0usize;

    while !ready.is_empty() {
        let wave = ready.clone();
        waves.push(wave.iter().map(|&i| &commands[i]).collect());
        processed += wave.len();
        ready.clear();

        for i in wave {
            for &j in &dependents[i] {
                in_degree[j] -= 1;
                if in_degree[j] == 0 {
                    ready.push(j);
                }
            }
        }
    }

    if processed != n {
        // Some commands were never enqueued ⇒ they form one or more cycles.
        let cycle = CycleFinder::new(commands).find();
        bail!(
            "Circular dependency detected among hook commands:\n  {}",
            cycle.join(" → ")
        );
    }

    Ok(waves)
}

// ---------------------------------------------------------------------------
// Cycle finder — DFS with three-colour marking
// Traverses the dependency graph (cmd → its deps) and returns the first
// cycle it finds as an ordered list of command names, closed at both ends:
// e.g. ["a", "b", "c", "a"].
// ---------------------------------------------------------------------------

struct CycleFinder<'a> {
    commands: &'a [Command],
    deps_of: Vec<Vec<usize>>,
    // 0 = unvisited, 1 = in DFS stack (grey), 2 = fully explored (black)
    color: Vec<u8>,
    stack: Vec<usize>,
}

impl<'a> CycleFinder<'a> {
    fn new(commands: &'a [Command]) -> Self {
        let name_to_idx: HashMap<&str, usize> = commands
            .iter()
            .enumerate()
            .map(|(i, c)| (c.name.as_str(), i))
            .collect();

        let deps_of = commands
            .iter()
            .map(|c| {
                c.depends
                    .iter()
                    .filter_map(|d| name_to_idx.get(d.as_str()).copied())
                    .collect()
            })
            .collect();

        CycleFinder {
            commands,
            deps_of,
            color: vec![0; commands.len()],
            stack: Vec::new(),
        }
    }

    fn find(&mut self) -> Vec<String> {
        for i in 0..self.commands.len() {
            if self.color[i] == 0 {
                if let Some(cycle) = self.dfs(i) {
                    return cycle;
                }
            }
        }
        // Shouldn't happen if called only when a cycle was detected by Kahn's.
        vec!["<cycle not reconstructed>".to_string()]
    }

    fn dfs(&mut self, u: usize) -> Option<Vec<String>> {
        self.color[u] = 1;
        self.stack.push(u);

        // Clone to avoid simultaneous mutable + immutable borrow of self.
        let deps = self.deps_of[u].clone();
        for v in deps {
            match self.color[v] {
                1 => {
                    // Back-edge: v is on the current stack → cycle found.
                    let cycle_start = self.stack.iter().position(|&x| x == v).unwrap();
                    let mut path: Vec<String> = self.stack[cycle_start..]
                        .iter()
                        .map(|&i| self.commands[i].name.clone())
                        .collect();
                    // Close the loop visually: "a → b → c → a"
                    path.push(self.commands[v].name.clone());
                    return Some(path);
                }
                0 => {
                    if let Some(cycle) = self.dfs(v) {
                        return Some(cycle);
                    }
                }
                _ => {} // already fully explored, no cycle through v
            }
        }

        self.stack.pop();
        self.color[u] = 2;
        None
    }
}

// ---------------------------------------------------------------------------
// Execution helpers
// ---------------------------------------------------------------------------

/// Run a single command, streaming its output directly to the terminal.
/// Used for sequential execution.
fn run_one(cmd: &Command, args: &[String], capture: bool) -> Result<Option<std::process::Output>> {
    if !capture {
        print!("  {} {} ... ", "▶".cyan(), cmd.name.bold());
    }

    if capture {
        let output = Cmd::new("sh")
            .arg("-c")
            .arg(&cmd.run)
            .arg("githops")
            .args(args)
            .envs(&cmd.env)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .output()?;
        return Ok(Some(output));
    }

    let status = Cmd::new("sh")
        .arg("-c")
        .arg(&cmd.run)
        .arg("githops")
        .args(args)
        .envs(&cmd.env)
        .status();

    match status {
        Ok(s) if s.success() => {
            println!("{}", "ok".green().bold());
            Ok(None)
        }
        Ok(s) => {
            println!("{}", "failed".red().bold());
            bail!(
                "Hook aborted: command '{}' exited with status {}.",
                cmd.name,
                s.code().unwrap_or(1)
            )
        }
        Err(e) => {
            println!("{}", "error".red().bold());
            bail!("Failed to execute '{}': {}", cmd.name, e)
        }
    }
}

/// Run all commands in a wave concurrently using scoped threads.
/// Waits for every thread to finish, then reports all failures at once.
fn run_wave_parallel(wave: &[&Command], args: &[String], cache_ctx: Option<&CacheCtx>) -> Result<()> {
    println!(
        "  {} running {} commands in parallel",
        "⇉".cyan().bold(),
        wave.len()
    );

    // Each thread writes its log lines here; printed atomically after joining.
    let failures: Mutex<Vec<String>> = Mutex::new(Vec::new());

    std::thread::scope(|scope| {
        for &cmd in wave {
            scope.spawn(|| {
                match run_one_cached(cmd, args, /*capture=*/ true, cache_ctx) {
                    Ok(Some(output)) => {
                        print_captured(cmd, &output);
                        if !output.status.success() {
                            let code = output.status.code().unwrap_or(1);
                            failures.lock().unwrap().push(format!(
                                "command '{}' exited with status {}",
                                cmd.name, code
                            ));
                        }
                    }
                    Ok(None) => {} // sequential success or cache hit
                    Err(e) => {
                        failures
                            .lock()
                            .unwrap()
                            .push(format!("command '{}': {}", cmd.name, e));
                    }
                }
            });
        }
    });

    let failures = failures.into_inner().unwrap();
    if !failures.is_empty() {
        bail!("Hook aborted — parallel failures:\n  • {}", failures.join("\n  • "));
    }

    Ok(())
}

/// Wrapper around [`run_one`] that checks and populates the cache when a
/// `CommandCache` config is present and caching is globally enabled.
fn run_one_cached(
    cmd: &Command,
    args: &[String],
    capture: bool,
    cache_ctx: Option<&CacheCtx>,
) -> Result<Option<std::process::Output>> {
    if let Some(ctx) = cache_ctx {
        if let Some(cache_cfg) = &cmd.cache {
            let paths = cache::expand_globs(&cache_cfg.inputs, &ctx.root);
            let inputs = cache::read_inputs(&paths);
            let key = cache::compute_key(&cmd.run, &cache_cfg.key, &inputs);

            if cache::is_hit(&key, &ctx.dir) {
                if !capture {
                    println!(
                        "  {} {} ... {}",
                        "▶".cyan(),
                        cmd.name.bold(),
                        "cached".dimmed()
                    );
                }
                // Treat as success without running.
                return Ok(None);
            }

            let result = run_one(cmd, args, capture)?;

            // Record the hit only when the command succeeded.
            let succeeded = match &result {
                None => true,
                Some(out) => out.status.success(),
            };
            if succeeded {
                let _ = cache::record_hit(&key, &ctx.dir);
            }

            return Ok(result);
        }
    }

    run_one(cmd, args, capture)
}

/// Print captured stdout/stderr with a per-command prefix so output from
/// concurrent commands doesn't interleave mid-line.
fn print_captured(cmd: &Command, output: &std::process::Output) {
    let ok = output.status.success();
    let status_label = if ok {
        "ok".green().bold()
    } else {
        "failed".red().bold()
    };

    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);

    let prefix = format!("  [{}]", cmd.name.bold());
    for line in stdout.lines() {
        println!("{} {}", prefix, line);
    }
    for line in stderr.lines() {
        eprintln!("{} {}", prefix, line.red());
    }
    println!("  {} {} {}", "▶".cyan(), cmd.name.bold(), status_label);
}
