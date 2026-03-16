use anyhow::Result;
use axum::{
    extract::{
        ws::{Message, WebSocket, WebSocketUpgrade},
        State,
    },
    http::header,
    response::{Html, IntoResponse, Response},
    routing::get,
    Router,
};
use colored::Colorize;
use std::collections::{BTreeMap, HashSet};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::SystemTime;

use githops_core::config::{
    Command, CommandCache, CommandEntry, Config, DefinitionEntry, GlobalCache, HookConfig, RefEntry,
    CONFIG_FILE,
};
use githops_core::git::hooks_dir;
use githops_core::hooks::ALL_HOOKS;

// ---------------------------------------------------------------------------
// Static UI assets (built by `pnpm run build` in ui/, embedded at compile time)
// ---------------------------------------------------------------------------

pub static INDEX_HTML: &str = include_str!("../ui/dist/index.html");
pub static APP_JS: &str = include_str!("../ui/dist/assets/app.js");
pub static APP_CSS: &str = include_str!("../ui/dist/assets/app.css");

// ---------------------------------------------------------------------------
// Entry point
// ---------------------------------------------------------------------------

pub fn run(open: bool) -> Result<()> {
    tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()?
        .block_on(async_run(open))
}

async fn async_run(open: bool) -> Result<()> {
    let config_path = Arc::new(std::env::current_dir()?.join(CONFIG_FILE));

    let listener = match tokio::net::TcpListener::bind("127.0.0.1:7890").await {
        Ok(l) => l,
        Err(_) => tokio::net::TcpListener::bind("127.0.0.1:0").await?,
    };
    let port = listener.local_addr()?.port();
    let url = format!("http://127.0.0.1:{}", port);

    println!("{} {}", "githops graph:".green().bold(), url.cyan().bold());
    println!(
        "  {}",
        "Press Ctrl+C to stop. Changes are saved to githops.yaml immediately.".dimmed()
    );

    if open {
        open_in_browser(&url);
    } else {
        println!(
            "  {} Use {} to open in browser.",
            "tip:".dimmed(),
            "githops graph --open".cyan()
        );
    }
    println!();

    let app = Router::new()
        .route("/", get(serve_html))
        .route("/assets/app.js", get(serve_js))
        .route("/assets/app.css", get(serve_css))
        .route("/ws", get(ws_handler))
        .with_state(config_path);

    axum::serve(listener, app).await?;
    Ok(())
}

// ---------------------------------------------------------------------------
// HTTP handlers
// ---------------------------------------------------------------------------

async fn serve_html() -> Html<&'static str> {
    Html(INDEX_HTML)
}

async fn serve_js() -> Response {
    (
        [(
            header::CONTENT_TYPE,
            "application/javascript; charset=utf-8",
        )],
        APP_JS,
    )
        .into_response()
}

async fn serve_css() -> Response {
    (
        [(header::CONTENT_TYPE, "text/css; charset=utf-8")],
        APP_CSS,
    )
        .into_response()
}

// ---------------------------------------------------------------------------
// WebSocket: CDP-style protocol
// ---------------------------------------------------------------------------

async fn ws_handler(
    ws: WebSocketUpgrade,
    State(config_path): State<Arc<PathBuf>>,
) -> Response {
    ws.on_upgrade(move |socket| ws_loop(socket, config_path))
}

fn config_mtime(path: &Path) -> Option<SystemTime> {
    path.metadata().ok()?.modified().ok()
}

async fn ws_loop(mut socket: WebSocket, config_path: Arc<PathBuf>) {
    if let Ok(json) = api_state(&config_path) {
        let event = format!(r#"{{"method":"state","params":{}}}"#, json);
        if socket.send(Message::Text(event)).await.is_err() {
            return;
        }
    }

    let mut last_mtime = config_mtime(&config_path);
    let mut interval = tokio::time::interval(tokio::time::Duration::from_secs(1));

    loop {
        tokio::select! {
            _ = interval.tick() => {
                let mtime = config_mtime(&config_path);
                if mtime != last_mtime {
                    last_mtime = mtime;
                    if let Ok(json) = api_state(&config_path) {
                        let event = format!(r#"{{"method":"state","params":{}}}"#, json);
                        if socket.send(Message::Text(event)).await.is_err() {
                            return;
                        }
                    }
                }
            }
            msg = socket.recv() => {
                match msg {
                    Some(Ok(Message::Text(text))) => {
                        let (response, push_state) = dispatch_ws(&text, &config_path);
                        if socket.send(Message::Text(response)).await.is_err() {
                            return;
                        }
                        if push_state {
                            last_mtime = config_mtime(&config_path);
                            if let Ok(json) = api_state(&config_path) {
                                let event = format!(r#"{{"method":"state","params":{}}}"#, json);
                                if socket.send(Message::Text(event)).await.is_err() {
                                    return;
                                }
                            }
                        }
                    }
                    Some(Ok(Message::Close(_))) | None => return,
                    _ => {}
                }
            }
        }
    }
}

fn dispatch_ws(text: &str, config_path: &Path) -> (String, bool) {
    #[derive(serde::Deserialize)]
    struct WsReq {
        id: u64,
        method: String,
        #[serde(default)]
        params: serde_json::Value,
    }

    let req = match serde_json::from_str::<WsReq>(text) {
        Ok(r) => r,
        Err(e) => {
            return (
                format!(r#"{{"id":0,"error":{{"message":"parse error: {}"}}}}"#, e),
                false,
            );
        }
    };

    let id = req.id;
    match handle_ws_request(&req.method, req.params, config_path) {
        Ok(result) => (
            serde_json::json!({"id": id, "result": result}).to_string(),
            true,
        ),
        Err(e) => (
            serde_json::json!({"id": id, "error": {"message": e.to_string()}}).to_string(),
            false,
        ),
    }
}

fn handle_ws_request(
    method: &str,
    params: serde_json::Value,
    config_path: &Path,
) -> Result<serde_json::Value> {
    match method {
        "hook.update" | "hook.remove" | "command.update" | "definition.update"
        | "definition.delete" => {
            let action = match method {
                "hook.update" => "update",
                "hook.remove" => "remove",
                "command.update" => "update-command",
                "definition.update" => "update-definition",
                "definition.delete" => "delete-definition",
                _ => unreachable!(),
            };
            let mut obj = match params {
                serde_json::Value::Object(m) => m,
                _ => serde_json::Map::new(),
            };
            obj.insert(
                "action".into(),
                serde_json::Value::String(action.to_string()),
            );
            let body = serde_json::to_vec(&serde_json::Value::Object(obj))?;
            api_update(&body, config_path)?;
            Ok(serde_json::json!({ "ok": true }))
        }
        "sync" => {
            let msg = api_sync(config_path)?;
            Ok(serde_json::json!({ "ok": true, "message": msg }))
        }
        "cache.clear" => {
            let config = if config_path.exists() {
                Config::load(config_path)?
            } else {
                Config::default()
            };
            let cache_dir = config.cache.cache_dir();
            let mut cleared = 0u32;
            if cache_dir.exists() {
                for entry in std::fs::read_dir(&cache_dir)?.flatten() {
                    if entry.path().extension().map(|x| x == "ok").unwrap_or(false) {
                        std::fs::remove_file(entry.path())?;
                        cleared += 1;
                    }
                }
            }
            Ok(serde_json::json!({ "ok": true, "cleared": cleared }))
        }
        "cache.update" => {
            let mut config = if config_path.exists() {
                Config::load(config_path)?
            } else {
                Config::default()
            };
            if let Some(enabled) = params.get("enabled").and_then(|v| v.as_bool()) {
                config.cache.enabled = enabled;
            }
            if let Some(dir_val) = params.get("dir") {
                config.cache.dir = dir_val
                    .as_str()
                    .filter(|s| !s.is_empty() && *s != ".githops/cache")
                    .map(|s| s.to_string());
            }
            // If nothing meaningful is set, reset to default (omitted from yaml)
            if !config.cache.enabled && config.cache.dir.is_none() {
                config.cache = GlobalCache::default();
            }
            config.save(config_path)?;
            Ok(serde_json::json!({ "ok": true }))
        }
        other => anyhow::bail!("unknown method: {other}"),
    }
}

// ---------------------------------------------------------------------------
// API logic
// ---------------------------------------------------------------------------

fn api_state(config_path: &Path) -> Result<String> {
    let config = if config_path.exists() {
        Config::load(config_path)?
    } else {
        Config::default()
    };
    let hooks_dir_path = hooks_dir().unwrap_or_else(|_| PathBuf::from(".git/hooks"));

    // ── Cache status ──────────────────────────────────────────────────────────
    let cache_dir = config.cache.cache_dir();
    let cache_dir_str = config.cache.dir.as_deref().unwrap_or(".githops/cache").to_string();
    let cache_entries: Vec<serde_json::Value> = if cache_dir.exists() {
        std::fs::read_dir(&cache_dir)
            .into_iter()
            .flatten()
            .flatten()
            .filter(|e| e.path().extension().map(|x| x == "ok").unwrap_or(false))
            .map(|e| {
                let key = e
                    .path()
                    .file_stem()
                    .unwrap_or_default()
                    .to_string_lossy()
                    .to_string();
                let age_ms = e
                    .metadata()
                    .ok()
                    .and_then(|m| m.modified().ok())
                    .and_then(|t| SystemTime::now().duration_since(t).ok())
                    .map(|d| d.as_millis() as u64)
                    .unwrap_or(0);
                serde_json::json!({ "key": key, "ageMs": age_ms })
            })
            .collect()
    } else {
        vec![]
    };

    let hook_states: Vec<serde_json::Value> = ALL_HOOKS
        .iter()
        .map(|info| {
            let installed = hooks_dir_path.join(info.name).exists();
            let cfg = config.hooks.get(info.name);
            let commands: Vec<serde_json::Value> = cfg
                .map(|c| {
                    c.commands
                        .iter()
                        .map(|entry| match entry {
                            CommandEntry::Ref(r) => {
                                let (def_name, def_run) = config
                                    .definitions
                                    .get(&r.r#ref)
                                    .and_then(|d| match d {
                                        DefinitionEntry::Single(cmd) => {
                                            Some((cmd.name.clone(), cmd.run.clone()))
                                        }
                                        _ => None,
                                    })
                                    .unwrap_or_else(|| (r.r#ref.clone(), String::new()));
                                serde_json::json!({
                                    "isRef":        true,
                                    "refName":      r.r#ref,
                                    "name":         r.name.as_deref().unwrap_or(&def_name),
                                    "nameOverride": r.name.as_deref().unwrap_or(""),
                                    "run":          def_run,
                                    "refArgs":      r.args.as_deref().unwrap_or(""),
                                    "depends": [],
                                    "env":     {},
                                    "test":    false,
                                })
                            }
                            CommandEntry::Inline(cmd) => serde_json::json!({
                                "isRef":   false,
                                "refName": "",
                                "name":    cmd.name,
                                "run":     cmd.run,
                                "depends": cmd.depends,
                                "env":     cmd.env,
                                "test":    cmd.test,
                                "cache":   cmd.cache.as_ref().map(|c| serde_json::json!({
                                    "inputs": c.inputs,
                                    "key":    c.key,
                                })),
                            }),
                        })
                        .collect()
                })
                .unwrap_or_default();

            serde_json::json!({
                "name":        info.name,
                "description": info.description,
                "category":    info.category.label(),
                "configured":  cfg.is_some(),
                "installed":   installed,
                "enabled":     cfg.map(|c| c.enabled).unwrap_or(false),
                "parallel":    cfg.map(|c| c.parallel).unwrap_or(false),
                "commands":    commands,
            })
        })
        .collect();

    let mut seen: HashSet<String> = HashSet::new();
    let mut unique_commands: Vec<serde_json::Value> = Vec::new();
    for hook_info in ALL_HOOKS {
        if let Some(cfg) = config.hooks.get(hook_info.name) {
            for entry in &cfg.commands {
                if let CommandEntry::Inline(cmd) = entry {
                    if seen.insert(cmd.name.clone()) {
                        let used_in: Vec<&str> = ALL_HOOKS
                            .iter()
                            .filter(|h| {
                                config
                                    .hooks
                                    .get(h.name)
                                    .map(|c| {
                                        c.commands.iter().any(|e| {
                                            if let CommandEntry::Inline(ic) = e {
                                                ic.name == cmd.name
                                            } else {
                                                false
                                            }
                                        })
                                    })
                                    .unwrap_or(false)
                            })
                            .map(|h| h.name)
                            .collect();
                        unique_commands.push(serde_json::json!({
                            "name":   cmd.name,
                            "run":    cmd.run,
                            "test":   cmd.test,
                            "usedIn": used_in,
                        }));
                    }
                }
            }
        }
    }

    let definitions: Vec<serde_json::Value> = config
        .definitions
        .iter()
        .map(|(name, def)| {
            let (def_type, cmds) = match def {
                DefinitionEntry::Single(cmd) => (
                    "single",
                    vec![serde_json::json!({
                        "name": cmd.name, "run": cmd.run,
                        "depends": cmd.depends, "env": cmd.env, "test": cmd.test,
                    })],
                ),
                DefinitionEntry::List(cmds) => (
                    "list",
                    cmds.iter()
                        .map(|cmd| {
                            serde_json::json!({
                                "name": cmd.name, "run": cmd.run,
                                "depends": cmd.depends, "env": cmd.env, "test": cmd.test,
                            })
                        })
                        .collect(),
                ),
            };
            serde_json::json!({ "name": name, "type": def_type, "commands": cmds })
        })
        .collect();

    Ok(serde_json::to_string(&serde_json::json!({
        "hooks":        hook_states,
        "commands":     unique_commands,
        "definitions":  definitions,
        "configExists": config_path.exists(),
        "cacheStatus": {
            "enabled": config.cache.enabled,
            "dir":     cache_dir_str,
            "entries": cache_entries,
        },
    }))?)
}

fn null_as_default<'de, D, T>(d: D) -> Result<T, D::Error>
where
    D: serde::Deserializer<'de>,
    T: Default + serde::Deserialize<'de>,
{
    use serde::Deserialize;
    Ok(Option::<T>::deserialize(d)?.unwrap_or_default())
}

#[derive(serde::Deserialize)]
struct UpdateRequest {
    action: String,
    #[serde(default, deserialize_with = "null_as_default")]
    hook: String,
    #[serde(default)]
    enabled: bool,
    #[serde(default)]
    parallel: bool,
    #[serde(default)]
    commands: Vec<CommandDto>,
    #[serde(default, rename = "oldName", deserialize_with = "null_as_default")]
    old_name: String,
    #[serde(default, deserialize_with = "null_as_default")]
    name: String,
    #[serde(default, deserialize_with = "null_as_default")]
    run: String,
    #[serde(default, rename = "defType", deserialize_with = "null_as_default")]
    def_type: String,
}

#[derive(serde::Deserialize, Default)]
struct CommandCacheDto {
    #[serde(default)]
    inputs: Vec<String>,
    #[serde(default)]
    key: Vec<String>,
}

#[derive(serde::Deserialize)]
struct CommandDto {
    #[serde(default, deserialize_with = "null_as_default")]
    name: String,
    #[serde(default, deserialize_with = "null_as_default")]
    run: String,
    #[serde(default)]
    depends: Vec<String>,
    #[serde(default)]
    env: BTreeMap<String, String>,
    #[serde(default)]
    test: bool,
    #[serde(default, rename = "isRef")]
    is_ref: bool,
    #[serde(default, rename = "refName", deserialize_with = "null_as_default")]
    ref_name: String,
    /// Extra arguments appended to the definition's run command (refs only).
    #[serde(default, rename = "refArgs", deserialize_with = "null_as_default")]
    ref_args: String,
    /// Explicit name override for this ref use-site (empty = use definition name).
    #[serde(default, rename = "nameOverride", deserialize_with = "null_as_default")]
    name_override: String,
    #[serde(default)]
    cache: Option<CommandCacheDto>,
}

impl CommandDto {
    fn into_cache(c: CommandCacheDto) -> CommandCache {
        CommandCache { inputs: c.inputs, key: c.key }
    }

    fn into_command(self) -> Command {
        Command {
            name: self.name,
            run: self.run,
            depends: self.depends,
            env: self.env,
            test: self.test,
            cache: self.cache.map(Self::into_cache),
        }
    }
    fn into_entry(self) -> CommandEntry {
        if self.is_ref {
            CommandEntry::Ref(RefEntry {
                r#ref: self.ref_name,
                args: if self.ref_args.is_empty() { None } else { Some(self.ref_args) },
                name: if self.name_override.is_empty() { None } else { Some(self.name_override) },
            })
        } else {
            let cache = self.cache.map(Self::into_cache);
            CommandEntry::Inline(Command {
                name: self.name,
                run: self.run,
                depends: self.depends,
                env: self.env,
                test: self.test,
                cache,
            })
        }
    }
}

fn api_update(body: &[u8], config_path: &Path) -> Result<()> {
    let req: UpdateRequest = serde_json::from_slice(body)?;
    let mut config = if config_path.exists() {
        Config::load(config_path)?
    } else {
        Config::default()
    };

    match req.action.as_str() {
        "update" => {
            let commands: Vec<CommandEntry> =
                req.commands.into_iter().map(|c| c.into_entry()).collect();
            let temp_cfg = HookConfig {
                enabled: req.enabled,
                parallel: req.parallel,
                commands: commands.clone(),
            };
            let resolved = temp_cfg.resolved_commands(&config.definitions);
            githops_core::config::validate_depends_pub(&resolved)?;
            config.hooks.set(
                &req.hook,
                HookConfig { enabled: req.enabled, parallel: req.parallel, commands },
            );
        }
        "remove" => {
            config.hooks.remove(&req.hook);
        }
        "update-command" => {
            if req.old_name.is_empty() {
                anyhow::bail!("oldName is required for update-command");
            }
            if req.name.is_empty() {
                anyhow::bail!("name is required for update-command");
            }
            update_command_in_all_hooks(&req.old_name, &req.name, &req.run, &mut config);
        }
        "update-definition" => {
            let def_name = req.name.trim().to_string();
            let old_name = req.old_name.trim().to_string();
            if def_name.is_empty() {
                anyhow::bail!("Definition name cannot be empty");
            }
            let entry = if req.def_type == "list" {
                let cmds: Vec<Command> =
                    req.commands.into_iter().map(|c| c.into_command()).collect();
                DefinitionEntry::List(cmds)
            } else {
                let cmd = req
                    .commands
                    .into_iter()
                    .next()
                    .map(|c| c.into_command())
                    .unwrap_or_else(|| Command {
                        name: def_name.clone(),
                        run: req.run,
                        depends: vec![],
                        env: BTreeMap::new(),
                        test: false,
                        cache: None,
                    });
                DefinitionEntry::Single(cmd)
            };
            if !old_name.is_empty() && old_name != def_name {
                config.definitions.remove(&old_name);
                update_def_ref_in_all_hooks(&old_name, &def_name, &mut config);
            }
            config.definitions.insert(def_name, entry);
        }
        "delete-definition" => {
            let def_name = req.name.trim().to_string();
            config.definitions.remove(&def_name);
            remove_def_refs_from_hooks(&def_name, &mut config);
        }
        other => anyhow::bail!("Unknown action: {other}"),
    }

    config.save(config_path)?;
    Ok(())
}

fn api_sync(config_path: &Path) -> Result<String> {
    let config = if config_path.exists() {
        Config::load(config_path)?
    } else {
        anyhow::bail!("No githops.yaml found. Run `githops init` first.");
    };
    let dir = hooks_dir()?;
    let (installed, skipped) = githops_core::sync_hooks::sync_to_hooks(&config, &dir, false)?;
    Ok(format!(
        "Synced {} hook(s){}",
        installed,
        if skipped > 0 {
            format!(" ({} skipped)", skipped)
        } else {
            String::new()
        }
    ))
}

// ---------------------------------------------------------------------------
// Config mutation helpers
// ---------------------------------------------------------------------------

fn update_command_in_all_hooks(
    old_name: &str,
    new_name: &str,
    new_run: &str,
    config: &mut Config,
) {
    let mut updates: Vec<(&'static str, HookConfig)> = Vec::new();
    for hook_info in ALL_HOOKS {
        let hook_cfg = match config.hooks.get(hook_info.name) {
            Some(cfg) => cfg.clone(),
            None => continue,
        };
        let mut changed = false;
        let mut new_commands = hook_cfg.commands.clone();
        for entry in &mut new_commands {
            if let CommandEntry::Inline(cmd) = entry {
                if cmd.name == old_name {
                    cmd.name = new_name.to_string();
                    if !new_run.is_empty() {
                        cmd.run = new_run.to_string();
                    }
                    changed = true;
                }
                for dep in &mut cmd.depends {
                    if dep == old_name {
                        *dep = new_name.to_string();
                        changed = true;
                    }
                }
            }
        }
        if changed {
            updates.push((
                hook_info.name,
                HookConfig {
                    enabled: hook_cfg.enabled,
                    parallel: hook_cfg.parallel,
                    commands: new_commands,
                },
            ));
        }
    }
    for (name, cfg) in updates {
        config.hooks.set(name, cfg);
    }
}

fn update_def_ref_in_all_hooks(old_name: &str, new_name: &str, config: &mut Config) {
    let mut updates: Vec<(&'static str, HookConfig)> = Vec::new();
    for hook_info in ALL_HOOKS {
        let hook_cfg = match config.hooks.get(hook_info.name) {
            Some(cfg) => cfg.clone(),
            None => continue,
        };
        let mut changed = false;
        let mut new_commands = hook_cfg.commands.clone();
        for entry in &mut new_commands {
            if let CommandEntry::Ref(r) = entry {
                if r.r#ref == old_name {
                    r.r#ref = new_name.to_string();
                    changed = true;
                }
            }
        }
        if changed {
            updates.push((
                hook_info.name,
                HookConfig {
                    enabled: hook_cfg.enabled,
                    parallel: hook_cfg.parallel,
                    commands: new_commands,
                },
            ));
        }
    }
    for (name, cfg) in updates {
        config.hooks.set(name, cfg);
    }
}

fn remove_def_refs_from_hooks(def_name: &str, config: &mut Config) {
    let mut updates: Vec<(&'static str, HookConfig)> = Vec::new();
    for hook_info in ALL_HOOKS {
        let hook_cfg = match config.hooks.get(hook_info.name) {
            Some(cfg) => cfg.clone(),
            None => continue,
        };
        let new_commands: Vec<_> = hook_cfg
            .commands
            .iter()
            .filter(|e| {
                if let CommandEntry::Ref(r) = e {
                    r.r#ref != def_name
                } else {
                    true
                }
            })
            .cloned()
            .collect();
        if new_commands.len() != hook_cfg.commands.len() {
            updates.push((
                hook_info.name,
                HookConfig {
                    enabled: hook_cfg.enabled,
                    parallel: hook_cfg.parallel,
                    commands: new_commands,
                },
            ));
        }
    }
    for (name, cfg) in updates {
        config.hooks.set(name, cfg);
    }
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn open_in_browser(url: &str) {
    #[cfg(target_os = "macos")]
    let _ = std::process::Command::new("open").arg(url).spawn();
    #[cfg(target_os = "linux")]
    let _ = std::process::Command::new("xdg-open").arg(url).spawn();
    #[cfg(target_os = "windows")]
    let _ = std::process::Command::new("cmd")
        .args(["/c", "start", "", url])
        .spawn();
}
