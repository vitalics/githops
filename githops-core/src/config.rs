use anyhow::{Context, Result};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

pub const CONFIG_FILE: &str = "githops.yaml";
pub const SCHEMA_FILE: &str = ".githops/githops.schema.json";

/// The JSON Schema for `githops.yaml`, embedded at compile time.
/// Use [`write_schema`] to materialize it on disk.
pub const SCHEMA_JSON: &str = include_str!("../githops.schema.json");

/// Write the embedded JSON Schema to `<dir>/.githops/githops.schema.json` if it is
/// absent or differs from the embedded version.
pub fn write_schema(dir: &std::path::Path) -> anyhow::Result<()> {
    let githops_dir = dir.join(".githops");
    std::fs::create_dir_all(&githops_dir)?;
    let path = dir.join(SCHEMA_FILE);
    let needs_write = match std::fs::read_to_string(&path) {
        Ok(existing) => existing != SCHEMA_JSON,
        Err(_) => true,
    };
    if needs_write {
        std::fs::write(&path, SCHEMA_JSON)?;
    }
    Ok(())
}

#[derive(Debug, Default, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "kebab-case")]
pub struct Config {
    /// Schema version
    #[serde(default = "default_version")]
    pub version: String,

    /// Reusable command definitions for YAML anchors.
    ///
    /// Define command templates here using YAML anchors (&name), then
    /// reference them with YAML aliases (*name) inside hook command lists.
    /// A list alias is automatically flattened into the parent sequence.
    ///
    /// Single command anchor:
    /// ```yaml
    /// definitions:
    ///   lint: &lint
    ///     name: lint
    ///     run: cargo clippy -- -D warnings
    /// ```
    ///
    /// List-of-commands anchor:
    /// ```yaml
    /// definitions:
    ///   quality: &quality
    ///     - name: lint
    ///       run: cargo clippy
    ///     - name: audit
    ///       run: cargo audit
    /// ```
    ///
    /// Usage in hooks (list aliases are inlined automatically):
    /// ```yaml
    /// hooks:
    ///   pre-commit:
    ///     commands:
    ///       - name: fmt
    ///         run: cargo fmt --check
    ///       - *lint        # single command
    ///       - *quality     # expands to two commands inline
    /// ```
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub definitions: BTreeMap<String, DefinitionEntry>,

    /// Hook definitions. Keys are git hook names (e.g. pre-commit, commit-msg).
    #[serde(default)]
    pub hooks: Hooks,

    /// Smart caching — skip commands whose inputs haven't changed.
    ///
    /// Set `enabled: true` here, then add a `cache.inputs` list to each
    /// command you want to cache.  The cache key is a SHA-256 of the command
    /// script, any extra `cache.key` strings, and the content of every input
    /// file.  A cache hit causes the command to be skipped with a "cached"
    /// message.  Cache entries are stored in `.githops/cache/` (or `cache.dir`).
    ///
    /// Example:
    /// ```yaml
    /// cache:
    ///   enabled: true
    ///
    /// hooks:
    ///   pre-commit:
    ///     commands:
    ///       - name: lint
    ///         run: cargo clippy -- -D warnings
    ///         cache:
    ///           inputs: ["src/**/*.rs", "Cargo.toml"]
    ///       - name: test
    ///         run: cargo test
    ///         cache:
    ///           inputs: ["src/**/*.rs", "tests/**/*.rs"]
    ///           key: ["$RUST_TOOLCHAIN"]
    /// ```
    #[serde(default, skip_serializing_if = "GlobalCache::is_unconfigured")]
    pub cache: GlobalCache,
}

/// Global cache settings.
#[derive(Debug, Default, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "kebab-case")]
pub struct GlobalCache {
    /// Enable caching.  Commands without a `cache` block are always executed.
    #[serde(default)]
    pub enabled: bool,

    /// Override the cache directory (default: `.githops/cache`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub dir: Option<String>,
}

impl GlobalCache {
    pub fn is_unconfigured(&self) -> bool {
        !self.enabled && self.dir.is_none()
    }

    pub fn cache_dir(&self) -> std::path::PathBuf {
        std::path::PathBuf::from(
            self.dir.as_deref().unwrap_or(".githops/cache"),
        )
    }
}

/// Per-command cache configuration.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "kebab-case")]
pub struct CommandCache {
    /// File glob patterns treated as inputs.
    /// The command is re-run only when the content of any matching file changes.
    /// Globs are relative to the repository root.  Example: `["src/**/*.rs", "Cargo.toml"]`
    #[serde(default)]
    pub inputs: Vec<String>,

    /// Extra strings mixed into the cache key (e.g. environment variable values
    /// or tool version strings).  Example: `["$RUST_TOOLCHAIN"]`
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub key: Vec<String>,
}

fn default_version() -> String {
    "1".to_string()
}

/// A reusable command definition: a single command mapping or a list of commands.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(untagged)]
pub enum DefinitionEntry {
    /// A list of commands that will be inlined when the anchor is used.
    List(Vec<Command>),
    /// A single command.
    Single(Command),
}

/// All supported git hooks. Configure any hook by adding its name as a key.
#[derive(Debug, Default, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "kebab-case")]
pub struct Hooks {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub applypatch_msg: Option<HookConfig>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub pre_applypatch: Option<HookConfig>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub post_applypatch: Option<HookConfig>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub pre_commit: Option<HookConfig>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub pre_merge_commit: Option<HookConfig>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub prepare_commit_msg: Option<HookConfig>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub commit_msg: Option<HookConfig>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub post_commit: Option<HookConfig>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub pre_rebase: Option<HookConfig>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub post_checkout: Option<HookConfig>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub post_merge: Option<HookConfig>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub pre_push: Option<HookConfig>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub pre_receive: Option<HookConfig>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub update: Option<HookConfig>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub proc_receive: Option<HookConfig>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub post_receive: Option<HookConfig>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub post_update: Option<HookConfig>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub reference_transaction: Option<HookConfig>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub push_to_checkout: Option<HookConfig>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub pre_auto_gc: Option<HookConfig>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub post_rewrite: Option<HookConfig>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub sendemail_validate: Option<HookConfig>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub fsmonitor_watchman: Option<HookConfig>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub p4_changelist: Option<HookConfig>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub p4_prepare_changelist: Option<HookConfig>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub p4_post_changelist: Option<HookConfig>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub p4_pre_submit: Option<HookConfig>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub post_index_change: Option<HookConfig>,
}

impl Hooks {
    /// Get a hook config by its git hook name (e.g. "pre-commit").
    pub fn get(&self, name: &str) -> Option<&HookConfig> {
        match name {
            "applypatch-msg" => self.applypatch_msg.as_ref(),
            "pre-applypatch" => self.pre_applypatch.as_ref(),
            "post-applypatch" => self.post_applypatch.as_ref(),
            "pre-commit" => self.pre_commit.as_ref(),
            "pre-merge-commit" => self.pre_merge_commit.as_ref(),
            "prepare-commit-msg" => self.prepare_commit_msg.as_ref(),
            "commit-msg" => self.commit_msg.as_ref(),
            "post-commit" => self.post_commit.as_ref(),
            "pre-rebase" => self.pre_rebase.as_ref(),
            "post-checkout" => self.post_checkout.as_ref(),
            "post-merge" => self.post_merge.as_ref(),
            "pre-push" => self.pre_push.as_ref(),
            "pre-receive" => self.pre_receive.as_ref(),
            "update" => self.update.as_ref(),
            "proc-receive" => self.proc_receive.as_ref(),
            "post-receive" => self.post_receive.as_ref(),
            "post-update" => self.post_update.as_ref(),
            "reference-transaction" => self.reference_transaction.as_ref(),
            "push-to-checkout" => self.push_to_checkout.as_ref(),
            "pre-auto-gc" => self.pre_auto_gc.as_ref(),
            "post-rewrite" => self.post_rewrite.as_ref(),
            "sendemail-validate" => self.sendemail_validate.as_ref(),
            "fsmonitor-watchman" => self.fsmonitor_watchman.as_ref(),
            "p4-changelist" => self.p4_changelist.as_ref(),
            "p4-prepare-changelist" => self.p4_prepare_changelist.as_ref(),
            "p4-post-changelist" => self.p4_post_changelist.as_ref(),
            "p4-pre-submit" => self.p4_pre_submit.as_ref(),
            "post-index-change" => self.post_index_change.as_ref(),
            _ => None,
        }
    }

    /// Set a hook config by its git hook name.
    pub fn set(&mut self, name: &str, cfg: HookConfig) {
        match name {
            "applypatch-msg" => self.applypatch_msg = Some(cfg),
            "pre-applypatch" => self.pre_applypatch = Some(cfg),
            "post-applypatch" => self.post_applypatch = Some(cfg),
            "pre-commit" => self.pre_commit = Some(cfg),
            "pre-merge-commit" => self.pre_merge_commit = Some(cfg),
            "prepare-commit-msg" => self.prepare_commit_msg = Some(cfg),
            "commit-msg" => self.commit_msg = Some(cfg),
            "post-commit" => self.post_commit = Some(cfg),
            "pre-rebase" => self.pre_rebase = Some(cfg),
            "post-checkout" => self.post_checkout = Some(cfg),
            "post-merge" => self.post_merge = Some(cfg),
            "pre-push" => self.pre_push = Some(cfg),
            "pre-receive" => self.pre_receive = Some(cfg),
            "update" => self.update = Some(cfg),
            "proc-receive" => self.proc_receive = Some(cfg),
            "post-receive" => self.post_receive = Some(cfg),
            "post-update" => self.post_update = Some(cfg),
            "reference-transaction" => self.reference_transaction = Some(cfg),
            "push-to-checkout" => self.push_to_checkout = Some(cfg),
            "pre-auto-gc" => self.pre_auto_gc = Some(cfg),
            "post-rewrite" => self.post_rewrite = Some(cfg),
            "sendemail-validate" => self.sendemail_validate = Some(cfg),
            "fsmonitor-watchman" => self.fsmonitor_watchman = Some(cfg),
            "p4-changelist" => self.p4_changelist = Some(cfg),
            "p4-prepare-changelist" => self.p4_prepare_changelist = Some(cfg),
            "p4-post-changelist" => self.p4_post_changelist = Some(cfg),
            "p4-pre-submit" => self.p4_pre_submit = Some(cfg),
            "post-index-change" => self.post_index_change = Some(cfg),
            _ => {}
        }
    }

    /// Remove a hook from the config by its git hook name.
    pub fn remove(&mut self, name: &str) {
        match name {
            "applypatch-msg" => self.applypatch_msg = None,
            "pre-applypatch" => self.pre_applypatch = None,
            "post-applypatch" => self.post_applypatch = None,
            "pre-commit" => self.pre_commit = None,
            "pre-merge-commit" => self.pre_merge_commit = None,
            "prepare-commit-msg" => self.prepare_commit_msg = None,
            "commit-msg" => self.commit_msg = None,
            "post-commit" => self.post_commit = None,
            "pre-rebase" => self.pre_rebase = None,
            "post-checkout" => self.post_checkout = None,
            "post-merge" => self.post_merge = None,
            "pre-push" => self.pre_push = None,
            "pre-receive" => self.pre_receive = None,
            "update" => self.update = None,
            "proc-receive" => self.proc_receive = None,
            "post-receive" => self.post_receive = None,
            "post-update" => self.post_update = None,
            "reference-transaction" => self.reference_transaction = None,
            "push-to-checkout" => self.push_to_checkout = None,
            "pre-auto-gc" => self.pre_auto_gc = None,
            "post-rewrite" => self.post_rewrite = None,
            "sendemail-validate" => self.sendemail_validate = None,
            "fsmonitor-watchman" => self.fsmonitor_watchman = None,
            "p4-changelist" => self.p4_changelist = None,
            "p4-prepare-changelist" => self.p4_prepare_changelist = None,
            "p4-post-changelist" => self.p4_post_changelist = None,
            "p4-pre-submit" => self.p4_pre_submit = None,
            "post-index-change" => self.post_index_change = None,
            _ => {}
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct HookConfig {
    /// Whether this hook is active. Set to false to temporarily disable.
    #[serde(default = "default_true")]
    pub enabled: bool,

    /// Run commands concurrently within each dependency wave.
    ///
    /// When `true`, commands that have no dependency relationship are started at
    /// the same time in separate threads.  Commands that share a `depends` link
    /// are still serialised — the dependent command waits until all its
    /// dependencies finish successfully.
    ///
    /// Use this to speed up independent checks (e.g. lint + test) while keeping
    /// ordered steps (e.g. build → deploy) sequential.
    ///
    /// Example:
    /// ```yaml
    /// hooks:
    ///   pre-push:
    ///     parallel: true
    ///     commands:
    ///       - name: lint
    ///         run: cargo clippy
    ///       - name: test
    ///         run: cargo test   # runs at the same time as lint
    /// ```
    #[serde(default)]
    pub parallel: bool,

    /// Commands to run when this hook fires.
    /// Each entry is either an inline command or a `$ref` to a definition.
    #[serde(default)]
    pub commands: Vec<CommandEntry>,
}

impl HookConfig {
    /// Resolve all command entries to concrete Commands by expanding `$ref` entries
    /// using the given definitions map. Unknown refs are silently skipped.
    pub fn resolved_commands<'a>(
        &'a self,
        definitions: &'a BTreeMap<String, DefinitionEntry>,
    ) -> Vec<Command> {
        let mut out = Vec::new();
        for entry in &self.commands {
            match entry {
                CommandEntry::Inline(cmd) => out.push(cmd.clone()),
                CommandEntry::Ref(r) => {
                    if let Some(def) = definitions.get(&r.r#ref) {
                        match def {
                            DefinitionEntry::Single(cmd) => {
                                let mut cmd = cmd.clone();
                                if let Some(args) = &r.args {
                                    cmd.run = format!("{} {}", cmd.run, args);
                                }
                                if let Some(name) = &r.name {
                                    cmd.name = name.clone();
                                }
                                out.push(cmd);
                            }
                            DefinitionEntry::List(cmds) => out.extend(cmds.iter().cloned()),
                        }
                    }
                }
            }
        }
        out
    }
}

fn default_true() -> bool {
    true
}

/// A command entry in a hook's command list: either an inline command definition
/// or a reference to a named definition (`$ref: name`).
///
/// The `$ref` form lets you reuse commands defined in the `definitions` section
/// without YAML anchors, so changes round-trip correctly through the UI editor.
///
/// Example using `$ref`:
/// ```yaml
/// hooks:
///   pre-commit:
///     commands:
///       - name: fmt
///         run: cargo fmt --check
///       - $ref: lint   # references definitions.lint
/// ```
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(untagged)]
pub enum CommandEntry {
    /// A reference to a named definition. Serialises as `{$ref: name}`.
    Ref(RefEntry),
    /// An inline command definition.
    Inline(Command),
}

impl From<Command> for CommandEntry {
    fn from(cmd: Command) -> Self {
        CommandEntry::Inline(cmd)
    }
}

/// A reference to a named definition in the `definitions` section.
///
/// Supports two optional overrides that are applied at the point of use,
/// without modifying the shared definition:
///
/// ```yaml
/// hooks:
///   pre-commit:
///     commands:
///       - $ref: lint                  # use definition as-is
///       - $ref: lint
///         args: "--fix"               # appends to the definition's run command
///         name: lint-fix              # overrides the display label
/// ```
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct RefEntry {
    /// Name of the definition to reference.
    #[serde(rename = "$ref")]
    pub r#ref: String,

    /// Extra arguments appended to the definition's `run` command.
    ///
    /// The final command executed is `{definition.run} {args}`.
    /// For example, if the definition runs `npm run lint`, setting
    /// `args: "--fix"` produces `npm run lint --fix`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub args: Option<String>,

    /// Override the display name shown in hook output for this specific use.
    /// When omitted, the definition's own `name` is used.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct Command {
    /// Human-readable label shown in output
    pub name: String,

    /// Shell command to execute. Hook arguments are available as $1, $2, etc.
    pub run: String,

    /// Names of commands in this hook that must complete successfully before
    /// this command starts. Forms a DAG; cycles are detected and rejected.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub depends: Vec<String>,

    /// Additional environment variables for this command
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub env: BTreeMap<String, String>,

    /// Mark this command as a test-only command (informational; not run during normal hooks).
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub test: bool,

    /// Cache configuration for this command.
    /// Requires `cache.enabled: true` in the top-level config.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cache: Option<CommandCache>,
}

// ---------------------------------------------------------------------------
// YAML pre-processor
// ---------------------------------------------------------------------------

fn flatten_command_aliases(root: &mut serde_yaml::Value) {
    let root_map = match root.as_mapping_mut() {
        Some(m) => m,
        None => return,
    };

    let hooks_key = serde_yaml::Value::String("hooks".into());
    let hooks = match root_map.get_mut(&hooks_key) {
        Some(h) => h,
        None => return,
    };
    let hooks_map = match hooks.as_mapping_mut() {
        Some(m) => m,
        None => return,
    };

    let hook_keys: Vec<serde_yaml::Value> = hooks_map.keys().cloned().collect();

    for hk in hook_keys {
        let hook_val = match hooks_map.get_mut(&hk) {
            Some(v) => v,
            None => continue,
        };
        let hook_map = match hook_val.as_mapping_mut() {
            Some(m) => m,
            None => continue,
        };

        let cmds_key = serde_yaml::Value::String("commands".into());
        let cmds_val = match hook_map.get_mut(&cmds_key) {
            Some(v) => v,
            None => continue,
        };
        let seq = match cmds_val.as_sequence_mut() {
            Some(s) => s,
            None => continue,
        };

        let original: Vec<serde_yaml::Value> = seq.drain(..).collect();
        for item in original {
            match item {
                serde_yaml::Value::Sequence(inner) => seq.extend(inner),
                other => seq.push(other),
            }
        }
    }
}

impl Config {
    pub fn load(path: &Path) -> Result<Self> {
        let content = std::fs::read_to_string(path)
            .with_context(|| format!("Failed to read {}", path.display()))?;

        let mut raw: serde_yaml::Value = serde_yaml::from_str(&content)
            .with_context(|| format!("Failed to parse YAML in {}", path.display()))?;

        flatten_command_aliases(&mut raw);

        serde_yaml::from_value(raw)
            .with_context(|| format!("Failed to deserialise {}", path.display()))
    }

    /// Find and load config from the current directory.
    pub fn find() -> Result<(Self, PathBuf)> {
        let path = Path::new(CONFIG_FILE);
        if path.exists() {
            return Ok((Self::load(path)?, path.to_path_buf()));
        }
        anyhow::bail!(
            "No {} found in the current directory. Run `githops init` first.",
            CONFIG_FILE
        )
    }

    pub fn save(&self, path: &Path) -> Result<()> {
        let yaml_body = serde_yaml::to_string(self)?;
        let content = if path.exists() {
            let existing = std::fs::read_to_string(path).unwrap_or_default();
            let first = existing.lines().next().unwrap_or("");
            if first.starts_with("# yaml-language-server:") {
                format!("{}\n{}", first, yaml_body)
            } else {
                yaml_body
            }
        } else {
            format!(
                "# yaml-language-server: $schema={}\n{}",
                SCHEMA_FILE, yaml_body
            )
        };
        std::fs::write(path, content)?;
        Ok(())
    }
}

// ---------------------------------------------------------------------------
// Validation (shared with graphui)
// ---------------------------------------------------------------------------

pub fn validate_depends_pub(commands: &[Command]) -> Result<()> {
    let names: std::collections::HashSet<&str> =
        commands.iter().map(|c| c.name.as_str()).collect();

    for cmd in commands {
        for dep in &cmd.depends {
            if !names.contains(dep.as_str()) {
                anyhow::bail!(
                    "Command '{}' depends on '{}', which is not defined in this hook.",
                    cmd.name,
                    dep
                );
            }
            if dep == &cmd.name {
                anyhow::bail!("Command '{}' depends on itself.", cmd.name);
            }
        }
    }
    Ok(())
}
