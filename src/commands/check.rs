use anyhow::{bail, Result};
use colored::Colorize;
use std::collections::HashMap;
use std::path::PathBuf;
use std::process::{Command as Cmd, Stdio};
use std::sync::Mutex;

use crate::config::{Command, Config};
use crate::hooks::find_hook;
use crate::logger::LogLayer;
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
        crate::log_error!(LogLayer::YamlExec, "unknown hook: {}", hook_name);
        bail!(
            "Unknown hook: '{}'. See `githops graph` for all supported hooks.",
            hook_name
        );
    }

    crate::log_verbose!(LogLayer::YamlResolve, "loading config");
    let (config, config_path) = Config::find()?;
    crate::log_info!(LogLayer::YamlResolve, "config loaded from {}", config_path.display());

    let hook_cfg = match config.hooks.get(hook_name) {
        Some(cfg) => cfg,
        None => {
            crate::log_verbose!(LogLayer::YamlResolve, "no config for hook '{}', skipping", hook_name);
            return Ok(());
        }
    };

    crate::log_verbose!(LogLayer::YamlResolve, "resolving commands for hook '{}'", hook_name);
    let commands = hook_cfg.resolved_commands_with_includes(&config.definitions, &config.include)?;
    crate::log_verbose!(LogLayer::YamlResolve, "resolved {} command(s)", commands.len());

    if !hook_cfg.enabled || commands.is_empty() {
        crate::log_verbose!(LogLayer::YamlExec, "hook '{}' is disabled or has no commands, skipping", hook_name);
        return Ok(());
    }

    // Filter out test-only commands during normal hook execution.
    let commands: Vec<_> = commands.into_iter().filter(|c| !c.test).collect();

    validate_depends_pub(&commands)?;

    let waves = build_execution_waves(&commands)?;
    crate::log_trace!(LogLayer::YamlExec, "built {} execution wave(s) for {} command(s)", waves.len(), commands.len());

    let cache_ctx = if config.cache.enabled {
        let root = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
        crate::log_verbose!(LogLayer::YamlExec, "cache enabled, dir: {}", config.cache.cache_dir().display());
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
            crate::log_verbose!(LogLayer::YamlExec, "running wave of {} commands in parallel", wave.len());
            run_wave_parallel(wave, args, cache_ctx.as_ref())?;
        } else {
            for cmd in wave {
                crate::log_verbose!(LogLayer::YamlExec, "running command '{}'", cmd.name);
                run_one_cached(cmd, args, /*capture=*/ false, cache_ctx.as_ref())?;
            }
        }
    }

    crate::log_info!(LogLayer::YamlExec, "hook '{}' completed successfully", hook_name);
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
    crate::log_trace!(LogLayer::YamlExec, "exec: {}", cmd.run);
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
                crate::log_trace!(LogLayer::YamlExec, "cache hit for '{}' (key: {})", cmd.name, &key[..8.min(key.len())]);
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

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeMap;

    fn cmd(name: &str, depends: &[&str]) -> Command {
        Command {
            name: name.to_string(),
            run: format!("echo {}", name),
            depends: depends.iter().map(|s| s.to_string()).collect(),
            env: BTreeMap::new(),
            test: false,
            cache: None,
        }
    }

    #[test]
    fn test_waves_no_dependencies_all_in_one_wave() {
        let cmds = vec![cmd("a", &[]), cmd("b", &[]), cmd("c", &[])];
        let waves = build_execution_waves(&cmds).unwrap();
        assert_eq!(waves.len(), 1);
        assert_eq!(waves[0].len(), 3);
    }

    #[test]
    fn test_waves_empty_commands() {
        let cmds: Vec<Command> = vec![];
        let waves = build_execution_waves(&cmds).unwrap();
        assert!(waves.is_empty());
    }

    #[test]
    fn test_waves_linear_chain() {
        // a → b → c: three separate waves
        let cmds = vec![
            cmd("a", &[]),
            cmd("b", &["a"]),
            cmd("c", &["b"]),
        ];
        let waves = build_execution_waves(&cmds).unwrap();
        assert_eq!(waves.len(), 3);
        assert_eq!(waves[0][0].name, "a");
        assert_eq!(waves[1][0].name, "b");
        assert_eq!(waves[2][0].name, "c");
    }

    #[test]
    fn test_waves_diamond_dag() {
        // a → b, a → c, b → d, c → d
        let cmds = vec![
            cmd("a", &[]),
            cmd("b", &["a"]),
            cmd("c", &["a"]),
            cmd("d", &["b", "c"]),
        ];
        let waves = build_execution_waves(&cmds).unwrap();
        // Wave 1: [a], Wave 2: [b, c], Wave 3: [d]
        assert_eq!(waves.len(), 3);
        assert_eq!(waves[0].len(), 1); // just a
        assert_eq!(waves[1].len(), 2); // b and c in parallel
        assert_eq!(waves[2].len(), 1); // just d
    }

    #[test]
    fn test_waves_detects_cycle() {
        // a → b → a (cycle)
        let cmds = vec![
            cmd("a", &["b"]),
            cmd("b", &["a"]),
        ];
        let result = build_execution_waves(&cmds);
        assert!(result.is_err());
        let msg = result.unwrap_err().to_string();
        assert!(msg.contains("Circular") || msg.contains("circular") || msg.contains("cycle"));
    }

    #[test]
    fn test_waves_single_command_no_deps() {
        let cmds = vec![cmd("lint", &[])];
        let waves = build_execution_waves(&cmds).unwrap();
        assert_eq!(waves.len(), 1);
        assert_eq!(waves[0][0].name, "lint");
    }

    #[test]
    fn test_waves_mixed_independent_and_dependent() {
        // a and b are independent; c depends on a only
        let cmds = vec![
            cmd("a", &[]),
            cmd("b", &[]),
            cmd("c", &["a"]),
        ];
        let waves = build_execution_waves(&cmds).unwrap();
        // Wave 1: [a, b], Wave 2: [c]
        assert_eq!(waves.len(), 2);
        let wave0_names: Vec<&str> = waves[0].iter().map(|c| c.name.as_str()).collect();
        assert!(wave0_names.contains(&"a"));
        assert!(wave0_names.contains(&"b"));
        assert_eq!(waves[1][0].name, "c");
    }

    #[test]
    fn test_waves_self_cycle_is_detected() {
        // a → a (self-loop)
        let cmds = vec![cmd("a", &["a"])];
        let result = build_execution_waves(&cmds);
        assert!(result.is_err());
    }

    #[test]
    fn test_cycle_finder_finds_simple_cycle() {
        let cmds = vec![
            cmd("a", &["b"]),
            cmd("b", &["a"]),
        ];
        let mut finder = CycleFinder::new(&cmds);
        let cycle = finder.find();
        // Cycle should include both a and b
        assert!(cycle.contains(&"a".to_string()) || cycle.contains(&"b".to_string()));
        // The cycle should close: first and last element the same
        assert_eq!(cycle.first(), cycle.last());
    }

    #[test]
    fn test_cycle_finder_finds_longer_cycle() {
        let cmds = vec![
            cmd("a", &["c"]),
            cmd("b", &["a"]),
            cmd("c", &["b"]),
        ];
        let mut finder = CycleFinder::new(&cmds);
        let cycle = finder.find();
        assert!(cycle.len() >= 4); // at least 3 nodes + closing node
        assert_eq!(cycle.first(), cycle.last());
    }

    #[test]
    fn test_validate_depends_pub_valid_chain() {
        let cmds = vec![
            cmd("a", &[]),
            cmd("b", &["a"]),
            cmd("c", &["b"]),
        ];
        assert!(validate_depends_pub(&cmds).is_ok());
    }

    #[test]
    fn test_validate_depends_pub_unknown_dep_errors() {
        let cmds = vec![cmd("b", &["nonexistent"])];
        let result = validate_depends_pub(&cmds);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("nonexistent"));
    }

    #[test]
    fn test_validate_depends_pub_self_dep_errors() {
        let cmds = vec![cmd("a", &["a"])];
        let result = validate_depends_pub(&cmds);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("itself"));
    }
}
