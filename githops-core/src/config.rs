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

/// Supported formats for included external config files.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "lowercase")]
pub enum IncludeType {
    /// JSON file (e.g. `package.json`)
    Json,
    /// TOML file (e.g. `Cargo.toml`)
    Toml,
    /// YAML file (e.g. `scripts.yaml`)
    Yaml,
}

/// An external file to import scripts from.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct LocalInclude {
    /// Path to the file, relative to the repository root.
    pub path: String,
    /// File format.
    #[serde(rename = "type")]
    pub file_type: IncludeType,
    /// Identifier used to reference this include in hook commands (`$include: <ref>`).
    #[serde(rename = "ref")]
    pub ref_name: String,
}

/// A remote file fetched via HTTP/HTTPS.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct RemoteInclude {
    /// Full URL of the file to fetch.
    pub url: String,
    /// File format. Defaults to `yaml`.
    #[serde(rename = "type", default = "default_yaml_type")]
    pub file_type: IncludeType,
    /// Identifier used to reference this include in hook commands (`$include: <ref>`).
    #[serde(rename = "ref")]
    pub ref_name: String,
}

fn default_yaml_type() -> IncludeType {
    IncludeType::Yaml
}

/// A file sourced from a remote Git repository at a specific revision.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct GitInclude {
    /// URL of the git repository (e.g. `https://github.com/org/repo.git`).
    pub url: String,
    /// Git revision to check out: branch name, tag, or full commit SHA.
    pub rev: String,
    /// Path of the file within the repository (e.g. `ci/scripts.yaml`).
    pub file: String,
    /// File format. Defaults to `yaml`.
    #[serde(rename = "type", default = "default_yaml_type")]
    pub file_type: IncludeType,
    /// Identifier used to reference this include in hook commands (`$include: <ref>`).
    #[serde(rename = "ref")]
    pub ref_name: String,
}

/// An entry in the `include` list. Supports `local` (filesystem), `remote` (HTTP/HTTPS),
/// and `git` (file from a remote Git repository) sources.
///
/// Example:
/// ```yaml
/// include:
///   - source: local
///     path: package.json
///     type: json
///     ref: packagejson
///   - source: remote
///     url: 'https://example.com/scripts.yaml'
///     ref: sharedscripts
///   - source: git
///     url: 'https://github.com/org/repo.git'
///     rev: main
///     file: 'ci/scripts.yaml'
///     ref: repotemplate
/// ```
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "source", rename_all = "lowercase")]
pub enum IncludeEntry {
    /// A local file relative to the repository root.
    Local(LocalInclude),
    /// A remote file fetched via HTTP/HTTPS.
    Remote(RemoteInclude),
    /// A file sourced from a remote Git repository.
    Git(GitInclude),
}

impl IncludeEntry {
    pub fn ref_name(&self) -> &str {
        match self {
            IncludeEntry::Local(l) => &l.ref_name,
            IncludeEntry::Remote(r) => &r.ref_name,
            IncludeEntry::Git(g) => &g.ref_name,
        }
    }
    pub fn path(&self) -> &str {
        match self {
            IncludeEntry::Local(l) => &l.path,
            IncludeEntry::Remote(r) => &r.url,
            IncludeEntry::Git(g) => &g.file,
        }
    }
    pub fn file_type(&self) -> &IncludeType {
        match self {
            IncludeEntry::Local(l) => &l.file_type,
            IncludeEntry::Remote(r) => &r.file_type,
            IncludeEntry::Git(g) => &g.file_type,
        }
    }
}

#[derive(Debug, Default, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "kebab-case")]
pub struct Config {
    /// Schema version
    #[serde(default = "default_version")]
    pub version: String,

    /// External file includes. Import scripts from `package.json`, `Cargo.toml`, etc.
    ///
    /// Defines named references to external files. Use `$include:` in hook commands to
    /// reference them.
    ///
    /// Example:
    /// ```yaml
    /// include:
    ///   - local:
    ///       path: package.json
    ///       type: json
    ///       ref: packagejson
    ///
    /// hooks:
    ///   pre-commit:
    ///     commands:
    ///       - $include: packagejson
    ///         args: scripts.lint
    /// ```
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub include: Vec<IncludeEntry>,

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
    /// `$include` entries are skipped here; use [`resolved_commands_with_includes`] to resolve them.
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
                CommandEntry::Include(_) => {} // resolved separately via resolved_commands_with_includes
            }
        }
        out
    }

    /// Like [`resolved_commands`] but also resolves `$include` entries by reading
    /// the referenced external files. Returns `Err` if an include cannot be resolved.
    pub fn resolved_commands_with_includes(
        &self,
        definitions: &BTreeMap<String, DefinitionEntry>,
        includes: &[IncludeEntry],
    ) -> Result<Vec<Command>> {
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
                CommandEntry::Include(inc_ref) => {
                    out.push(resolve_include_entry(inc_ref, includes)?);
                }
            }
        }
        Ok(out)
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
    /// A reference to a value in an included external file.
    Include(IncludeRef),
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

/// A reference to a value in an included external file, used as a hook command.
///
/// The value at `run` (dot-notation path) is extracted from the file and used as the
/// shell command. Optional `args` are appended to that command. Optional `env` sets
/// environment variables for the invocation.
///
/// Example:
/// ```yaml
/// hooks:
///   pre-commit:
///     commands:
///       - $include: packagejson   # references include with ref: packagejson
///         run: scripts.lint       # navigates to scripts → lint in the file
///         args: "--fix"           # appended to the resolved command
///         env:
///           NODE_ENV: production
///         name: lint              # optional display name (defaults to last segment of run)
/// ```
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct IncludeRef {
    /// Name of the include to reference (must match an `IncludeEntry` with this `ref`).
    #[serde(rename = "$include")]
    pub include_ref: String,

    /// Dot-notation path to the value in the file (e.g. `scripts.lint`).
    pub run: String,

    /// Extra arguments appended to the resolved command string.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub args: Option<String>,

    /// Environment variables set for this command invocation.
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub env: BTreeMap<String, String>,

    /// Optional display name. Defaults to the last segment of `run`.
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
// Include resolution
// ---------------------------------------------------------------------------

/// Resolve a `$include` command entry by reading the referenced external file
/// and navigating the dot-notation path specified in `run`.
///
/// If `args` is set, it is appended to the resolved command string.
/// Supports local files, remote HTTP/HTTPS files, and files from remote Git repositories.
pub fn resolve_include_entry(inc_ref: &IncludeRef, includes: &[IncludeEntry]) -> Result<Command> {
    let entry = includes
        .iter()
        .find(|e| e.ref_name() == inc_ref.include_ref)
        .ok_or_else(|| anyhow::anyhow!(
            "Include '{}' not defined in the `include:` section.",
            inc_ref.include_ref
        ))?;

    let (content, file_type) = fetch_include_content(entry)?;

    let base_run = match file_type {
        IncludeType::Json => {
            let json: serde_json::Value = serde_json::from_str(&content)
                .with_context(|| format!("Failed to parse JSON from '{}'", entry.path()))?;
            navigate_json(&json, &inc_ref.run)
                .with_context(|| format!("Path '{}' not found in '{}'", inc_ref.run, entry.path()))?
        }
        IncludeType::Toml => {
            let toml_val: toml::Value = toml::from_str(&content)
                .with_context(|| format!("Failed to parse TOML from '{}'", entry.path()))?;
            navigate_toml(&toml_val, &inc_ref.run)
                .with_context(|| format!("Path '{}' not found in '{}'", inc_ref.run, entry.path()))?
        }
        IncludeType::Yaml => {
            let yaml_val: serde_yaml::Value = serde_yaml::from_str(&content)
                .with_context(|| format!("Failed to parse YAML from '{}'", entry.path()))?;
            navigate_yaml(&yaml_val, &inc_ref.run)
                .with_context(|| format!("Path '{}' not found in '{}'", inc_ref.run, entry.path()))?
        }
    };

    let run = match &inc_ref.args {
        Some(extra) if !extra.is_empty() => format!("{} {}", base_run, extra),
        _ => base_run,
    };

    let name = inc_ref.name.clone().unwrap_or_else(|| {
        inc_ref.run.split('.').last().unwrap_or(&inc_ref.run).to_string()
    });

    Ok(Command {
        name,
        run,
        depends: vec![],
        env: inc_ref.env.clone(),
        test: false,
        cache: None,
    })
}

/// Fetch the content string and resolved file type for any include source.
fn fetch_include_content(entry: &IncludeEntry) -> Result<(String, &IncludeType)> {
    match entry {
        IncludeEntry::Local(l) => {
            let content = std::fs::read_to_string(&l.path)
                .with_context(|| format!("Failed to read include file '{}'", l.path))?;
            Ok((content, &l.file_type))
        }
        IncludeEntry::Remote(r) => {
            let content = ureq::get(&r.url)
                .call()
                .with_context(|| format!("Failed to fetch remote include '{}'", r.url))?
                .into_string()
                .with_context(|| format!("Failed to read response body from '{}'", r.url))?;
            Ok((content, &r.file_type))
        }
        IncludeEntry::Git(g) => {
            let content = fetch_git_file(&g.url, &g.rev, &g.file)?;
            Ok((content, &g.file_type))
        }
    }
}

/// Clone a git repository at the given revision and read a single file from it.
/// Uses a temporary directory that is cleaned up after the file is read.
fn fetch_git_file(url: &str, rev: &str, file: &str) -> Result<String> {
    use std::process::Command as Cmd;

    // Build a deterministic temp path from the url+rev hash to allow basic re-use
    // within the same process run, but always re-clone for correctness.
    let hash = {
        use sha2::{Digest, Sha256};
        let mut h = Sha256::new();
        h.update(url.as_bytes());
        h.update(rev.as_bytes());
        format!("{:x}", h.finalize())[..12].to_string()
    };
    let tmp_dir = std::env::temp_dir().join(format!("githops-git-{}", hash));

    // Re-use existing clone if present (avoids duplicate clones in the same run).
    if !tmp_dir.exists() {
        let status = Cmd::new("git")
            .args([
                "clone",
                "--depth=1",
                "--branch", rev,
                "--",
                url,
                tmp_dir.to_str().unwrap_or("/tmp/githops-git"),
            ])
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .status()
            .with_context(|| format!("Failed to run `git clone` for '{}'", url))?;

        if !status.success() {
            anyhow::bail!(
                "git clone failed for '{}' at revision '{}'. \
                 Make sure the URL is accessible and the revision exists.",
                url, rev
            );
        }
    }

    let file_path = tmp_dir.join(file);
    let content = std::fs::read_to_string(&file_path)
        .with_context(|| format!("File '{}' not found in git repository '{}'", file, url))?;

    Ok(content)
}

fn navigate_json(value: &serde_json::Value, path: &str) -> Result<String> {
    let mut current = value;
    for key in path.split('.') {
        current = current
            .get(key)
            .ok_or_else(|| anyhow::anyhow!("Key '{}' not found", key))?;
    }
    match current {
        serde_json::Value::String(s) => Ok(s.clone()),
        other => Ok(other.to_string()),
    }
}

fn navigate_toml(value: &toml::Value, path: &str) -> Result<String> {
    let mut current = value;
    for key in path.split('.') {
        current = current
            .get(key)
            .ok_or_else(|| anyhow::anyhow!("Key '{}' not found", key))?;
    }
    match current {
        toml::Value::String(s) => Ok(s.clone()),
        other => Ok(other.to_string()),
    }
}

fn navigate_yaml(value: &serde_yaml::Value, path: &str) -> Result<String> {
    let mut current = value;
    for key in path.split('.') {
        current = current
            .get(key)
            .ok_or_else(|| anyhow::anyhow!("Key '{}' not found", key))?;
    }
    match current {
        serde_yaml::Value::String(s) => Ok(s.clone()),
        serde_yaml::Value::Number(n) => Ok(n.to_string()),
        other => {
            serde_yaml::to_string(other)
                .map(|s| s.trim().to_string())
                .map_err(|e| anyhow::anyhow!("Cannot convert YAML value to string: {}", e))
        }
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

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeMap;

    // -----------------------------------------------------------------------
    // Config parsing tests
    // -----------------------------------------------------------------------

    #[test]
    fn test_parse_minimal_config() {
        let yaml = r#"
hooks:
  pre-commit:
    enabled: true
    commands:
      - name: lint
        run: echo lint
"#;
        let config: Config = serde_yaml::from_str(yaml).unwrap();
        let hook = config.hooks.pre_commit.as_ref().unwrap();
        assert!(hook.enabled);
        assert_eq!(hook.commands.len(), 1);
    }

    #[test]
    fn test_parse_config_with_definitions() {
        let yaml = r#"
definitions:
  lint:
    name: ESLint
    run: npx eslint .

hooks:
  pre-commit:
    commands:
      - $ref: lint
"#;
        let config: Config = serde_yaml::from_str(yaml).unwrap();
        assert!(config.definitions.contains_key("lint"));
        let hook = config.hooks.pre_commit.as_ref().unwrap();
        assert_eq!(hook.commands.len(), 1);
        assert!(matches!(hook.commands[0], CommandEntry::Ref(_)));
    }

    #[test]
    fn test_parse_config_with_local_include() {
        let yaml = "include:\n  - source: local\n    path: package.json\n    type: json\n    ref: pkg\nhooks:\n  pre-commit:\n    commands:\n      - $include: pkg\n        run: scripts.lint\n";
        let config: Config = serde_yaml::from_str(yaml).unwrap();
        assert_eq!(config.include.len(), 1);
        assert!(matches!(config.include[0], IncludeEntry::Local(_)));
        assert_eq!(config.include[0].ref_name(), "pkg");
        let hook = config.hooks.pre_commit.as_ref().unwrap();
        assert!(matches!(hook.commands[0], CommandEntry::Include(_)));
    }

    #[test]
    fn test_parse_config_with_remote_include() {
        let yaml = "include:\n  - source: remote\n    url: 'https://example.com/scripts.yaml'\n    type: yaml\n    ref: remote1\n";
        let config: Config = serde_yaml::from_str(yaml).unwrap();
        assert_eq!(config.include.len(), 1);
        assert!(matches!(config.include[0], IncludeEntry::Remote(_)));
        assert_eq!(config.include[0].ref_name(), "remote1");
    }

    #[test]
    fn test_parse_config_with_git_include() {
        let yaml = "include:\n  - source: git\n    url: 'https://github.com/org/repo.git'\n    rev: main\n    file: ci/scripts.yaml\n    ref: repo1\n";
        let config: Config = serde_yaml::from_str(yaml).unwrap();
        assert_eq!(config.include.len(), 1);
        assert!(matches!(config.include[0], IncludeEntry::Git(_)));
        assert_eq!(config.include[0].ref_name(), "repo1");
        if let IncludeEntry::Git(g) = &config.include[0] {
            assert_eq!(g.rev, "main");
            assert_eq!(g.file, "ci/scripts.yaml");
        }
    }

    #[test]
    fn test_include_entry_ref_name_accessor() {
        let local = IncludeEntry::Local(LocalInclude {
            path: "pkg.json".into(),
            file_type: IncludeType::Json,
            ref_name: "mypkg".into(),
        });
        assert_eq!(local.ref_name(), "mypkg");
        assert_eq!(local.path(), "pkg.json");
        assert!(matches!(local.file_type(), IncludeType::Json));
    }

    // -----------------------------------------------------------------------
    // CommandEntry deserialization tests
    // -----------------------------------------------------------------------

    #[test]
    fn test_command_entry_inline_deser() {
        let yaml = r#"name: lint
run: npx eslint ."#;
        let entry: CommandEntry = serde_yaml::from_str(yaml).unwrap();
        assert!(matches!(entry, CommandEntry::Inline(_)));
        if let CommandEntry::Inline(cmd) = entry {
            assert_eq!(cmd.name, "lint");
            assert_eq!(cmd.run, "npx eslint .");
        }
    }

    #[test]
    fn test_command_entry_ref_deser() {
        let yaml = r#"$ref: lint"#;
        let entry: CommandEntry = serde_yaml::from_str(yaml).unwrap();
        assert!(matches!(entry, CommandEntry::Ref(_)));
        if let CommandEntry::Ref(r) = entry {
            assert_eq!(r.r#ref, "lint");
            assert!(r.args.is_none());
        }
    }

    #[test]
    fn test_command_entry_ref_with_args_deser() {
        let yaml = "$ref: lint\nargs: \"--fix\"";
        let entry: CommandEntry = serde_yaml::from_str(yaml).unwrap();
        if let CommandEntry::Ref(r) = entry {
            assert_eq!(r.args.as_deref(), Some("--fix"));
        } else {
            panic!("Expected Ref variant");
        }
    }

    #[test]
    fn test_command_entry_include_deser() {
        let yaml = "$include: mypkg\nrun: scripts.lint";
        let entry: CommandEntry = serde_yaml::from_str(yaml).unwrap();
        assert!(matches!(entry, CommandEntry::Include(_)));
        if let CommandEntry::Include(inc) = entry {
            assert_eq!(inc.include_ref, "mypkg");
            assert_eq!(inc.run, "scripts.lint");
            assert!(inc.args.is_none());
            assert!(inc.name.is_none());
        }
    }

    #[test]
    fn test_command_entry_include_with_args_deser() {
        let yaml = "$include: mypkg\nrun: scripts.lint\nargs: \"--fix\"";
        let entry: CommandEntry = serde_yaml::from_str(yaml).unwrap();
        if let CommandEntry::Include(inc) = entry {
            assert_eq!(inc.run, "scripts.lint");
            assert_eq!(inc.args.as_deref(), Some("--fix"));
        } else {
            panic!("Expected Include variant");
        }
    }

    #[test]
    fn test_command_entry_include_with_name_deser() {
        let yaml = "$include: mypkg\nrun: scripts.lint\nname: ESLint";
        let entry: CommandEntry = serde_yaml::from_str(yaml).unwrap();
        if let CommandEntry::Include(inc) = entry {
            assert_eq!(inc.name.as_deref(), Some("ESLint"));
        } else {
            panic!("Expected Include variant");
        }
    }

    // -----------------------------------------------------------------------
    // resolved_commands tests
    // -----------------------------------------------------------------------

    #[test]
    fn test_resolved_commands_inline_only() {
        let hook = HookConfig {
            enabled: true,
            parallel: false,
            commands: vec![
                CommandEntry::Inline(Command {
                    name: "lint".into(),
                    run: "echo lint".into(),
                    depends: vec![],
                    env: BTreeMap::new(),
                    test: false,
                    cache: None,
                }),
            ],
        };
        let resolved = hook.resolved_commands(&BTreeMap::new());
        assert_eq!(resolved.len(), 1);
        assert_eq!(resolved[0].name, "lint");
    }

    #[test]
    fn test_resolved_commands_ref_expansion() {
        let mut defs = BTreeMap::new();
        defs.insert("lint".to_string(), DefinitionEntry::Single(Command {
            name: "lint".into(),
            run: "npx eslint .".into(),
            depends: vec![],
            env: BTreeMap::new(),
            test: false,
            cache: None,
        }));
        let hook = HookConfig {
            enabled: true,
            parallel: false,
            commands: vec![
                CommandEntry::Ref(RefEntry { r#ref: "lint".into(), args: None, name: None }),
            ],
        };
        let resolved = hook.resolved_commands(&defs);
        assert_eq!(resolved.len(), 1);
        assert_eq!(resolved[0].run, "npx eslint .");
    }

    #[test]
    fn test_resolved_commands_ref_with_args() {
        let mut defs = BTreeMap::new();
        defs.insert("lint".to_string(), DefinitionEntry::Single(Command {
            name: "lint".into(),
            run: "npx eslint .".into(),
            depends: vec![],
            env: BTreeMap::new(),
            test: false,
            cache: None,
        }));
        let hook = HookConfig {
            enabled: true,
            parallel: false,
            commands: vec![
                CommandEntry::Ref(RefEntry {
                    r#ref: "lint".into(),
                    args: Some("--fix".into()),
                    name: None,
                }),
            ],
        };
        let resolved = hook.resolved_commands(&defs);
        assert_eq!(resolved[0].run, "npx eslint . --fix");
    }

    #[test]
    fn test_resolved_commands_ref_with_name_override() {
        let mut defs = BTreeMap::new();
        defs.insert("lint".to_string(), DefinitionEntry::Single(Command {
            name: "lint".into(),
            run: "npx eslint .".into(),
            depends: vec![],
            env: BTreeMap::new(),
            test: false,
            cache: None,
        }));
        let hook = HookConfig {
            enabled: true,
            parallel: false,
            commands: vec![
                CommandEntry::Ref(RefEntry {
                    r#ref: "lint".into(),
                    args: None,
                    name: Some("ESLint (fix)".into()),
                }),
            ],
        };
        let resolved = hook.resolved_commands(&defs);
        assert_eq!(resolved[0].name, "ESLint (fix)");
    }

    #[test]
    fn test_resolved_commands_list_definition() {
        let mut defs = BTreeMap::new();
        defs.insert("quality".to_string(), DefinitionEntry::List(vec![
            Command {
                name: "lint".into(),
                run: "echo lint".into(),
                depends: vec![],
                env: BTreeMap::new(),
                test: false,
                cache: None,
            },
            Command {
                name: "test".into(),
                run: "echo test".into(),
                depends: vec![],
                env: BTreeMap::new(),
                test: false,
                cache: None,
            },
        ]));
        let hook = HookConfig {
            enabled: true,
            parallel: false,
            commands: vec![
                CommandEntry::Ref(RefEntry { r#ref: "quality".into(), args: None, name: None }),
            ],
        };
        let resolved = hook.resolved_commands(&defs);
        assert_eq!(resolved.len(), 2);
    }

    #[test]
    fn test_resolved_commands_skips_include_entries() {
        let hook = HookConfig {
            enabled: true,
            parallel: false,
            commands: vec![
                CommandEntry::Include(IncludeRef {
                    include_ref: "pkg".into(),
                    run: "scripts.lint".into(),
                    args: None,
                    env: BTreeMap::new(),
                    name: None,
                }),
                CommandEntry::Inline(Command {
                    name: "fmt".into(),
                    run: "echo fmt".into(),
                    depends: vec![],
                    env: BTreeMap::new(),
                    test: false,
                    cache: None,
                }),
            ],
        };
        let resolved = hook.resolved_commands(&BTreeMap::new());
        assert_eq!(resolved.len(), 1);
        assert_eq!(resolved[0].name, "fmt");
    }

    #[test]
    fn test_resolved_commands_with_includes_local() {
        use std::io::Write;
        use tempfile::NamedTempFile;

        let mut f = NamedTempFile::new().unwrap();
        writeln!(f, r#"{{"scripts": {{"lint": "eslint . --ext .ts"}}}}"#).unwrap();
        let path = f.path().to_str().unwrap().to_string();

        let includes = vec![IncludeEntry::Local(LocalInclude {
            path: path.clone(),
            file_type: IncludeType::Json,
            ref_name: "pkg".into(),
        })];

        let hook = HookConfig {
            enabled: true,
            parallel: false,
            commands: vec![
                CommandEntry::Include(IncludeRef {
                    include_ref: "pkg".into(),
                    run: "scripts.lint".into(),
                    args: None,
                    env: BTreeMap::new(),
                    name: None,
                }),
            ],
        };

        let resolved = hook.resolved_commands_with_includes(&BTreeMap::new(), &includes).unwrap();
        assert_eq!(resolved.len(), 1);
        assert_eq!(resolved[0].name, "lint");
        assert_eq!(resolved[0].run, "eslint . --ext .ts");
    }

    #[test]
    fn test_resolved_commands_with_includes_name_override() {
        use std::io::Write;
        use tempfile::NamedTempFile;

        let mut f = NamedTempFile::new().unwrap();
        writeln!(f, r#"{{"scripts": {{"lint": "eslint ."}}}}"#).unwrap();
        let path = f.path().to_str().unwrap().to_string();

        let includes = vec![IncludeEntry::Local(LocalInclude {
            path,
            file_type: IncludeType::Json,
            ref_name: "pkg".into(),
        })];

        let hook = HookConfig {
            enabled: true,
            parallel: false,
            commands: vec![
                CommandEntry::Include(IncludeRef {
                    include_ref: "pkg".into(),
                    run: "scripts.lint".into(),
                    args: None,
                    env: BTreeMap::new(),
                    name: Some("ESLint".into()),
                }),
            ],
        };

        let resolved = hook.resolved_commands_with_includes(&BTreeMap::new(), &includes).unwrap();
        assert_eq!(resolved[0].name, "ESLint");
    }

    #[test]
    fn test_resolved_commands_with_includes_unknown_ref_errors() {
        let includes: Vec<IncludeEntry> = vec![];
        let hook = HookConfig {
            enabled: true,
            parallel: false,
            commands: vec![
                CommandEntry::Include(IncludeRef {
                    include_ref: "nonexistent".into(),
                    run: "scripts.lint".into(),
                    args: None,
                    env: BTreeMap::new(),
                    name: None,
                }),
            ],
        };
        let result = hook.resolved_commands_with_includes(&BTreeMap::new(), &includes);
        assert!(result.is_err());
        let msg = result.unwrap_err().to_string();
        assert!(msg.contains("nonexistent"));
    }

    // -----------------------------------------------------------------------
    // Navigation function tests
    // -----------------------------------------------------------------------

    #[test]
    fn test_navigate_json_simple() {
        let json: serde_json::Value =
            serde_json::from_str(r#"{"scripts": {"lint": "eslint ."}}"#).unwrap();
        let result = navigate_json(&json, "scripts.lint").unwrap();
        assert_eq!(result, "eslint .");
    }

    #[test]
    fn test_navigate_json_top_level() {
        let json: serde_json::Value = serde_json::from_str(r#"{"name": "myapp"}"#).unwrap();
        assert_eq!(navigate_json(&json, "name").unwrap(), "myapp");
    }

    #[test]
    fn test_navigate_json_missing_key() {
        let json: serde_json::Value = serde_json::from_str(r#"{"scripts": {}}"#).unwrap();
        assert!(navigate_json(&json, "scripts.lint").is_err());
    }

    #[test]
    fn test_navigate_json_deeply_nested() {
        let json: serde_json::Value =
            serde_json::from_str(r#"{"a": {"b": {"c": "deep"}}}"#).unwrap();
        assert_eq!(navigate_json(&json, "a.b.c").unwrap(), "deep");
    }

    #[test]
    fn test_navigate_toml_simple() {
        let toml_val: toml::Value =
            toml::from_str("[scripts]\nlint = \"cargo clippy\"").unwrap();
        assert_eq!(navigate_toml(&toml_val, "scripts.lint").unwrap(), "cargo clippy");
    }

    #[test]
    fn test_navigate_toml_missing_key() {
        let toml_val: toml::Value = toml::from_str("[scripts]\n").unwrap();
        assert!(navigate_toml(&toml_val, "scripts.missing").is_err());
    }

    #[test]
    fn test_navigate_yaml_simple() {
        let yaml_val: serde_yaml::Value =
            serde_yaml::from_str("scripts:\n  lint: \"npm run lint\"").unwrap();
        assert_eq!(navigate_yaml(&yaml_val, "scripts.lint").unwrap(), "npm run lint");
    }

    #[test]
    fn test_navigate_yaml_missing_key() {
        let yaml_val: serde_yaml::Value = serde_yaml::from_str("scripts: {}").unwrap();
        assert!(navigate_yaml(&yaml_val, "scripts.missing").is_err());
    }

    // -----------------------------------------------------------------------
    // resolve_include_entry tests
    // -----------------------------------------------------------------------

    #[test]
    fn test_resolve_include_local_json() {
        use std::io::Write;
        use tempfile::NamedTempFile;

        let mut f = NamedTempFile::new().unwrap();
        write!(f, r#"{{"scripts": {{"test": "jest"}}}}"#).unwrap();

        let includes = vec![IncludeEntry::Local(LocalInclude {
            path: f.path().to_str().unwrap().to_string(),
            file_type: IncludeType::Json,
            ref_name: "pkg".into(),
        })];
        let inc_ref = IncludeRef {
            include_ref: "pkg".into(),
            run: "scripts.test".into(),
            args: None,
            env: BTreeMap::new(),
            name: None,
        };
        let cmd = resolve_include_entry(&inc_ref, &includes).unwrap();
        assert_eq!(cmd.run, "jest");
        assert_eq!(cmd.name, "test");
    }

    #[test]
    fn test_resolve_include_local_toml() {
        use std::io::Write;
        use tempfile::NamedTempFile;

        let mut f = NamedTempFile::new().unwrap();
        write!(f, "[scripts]\nbuild = \"cargo build --release\"").unwrap();

        let includes = vec![IncludeEntry::Local(LocalInclude {
            path: f.path().to_str().unwrap().to_string(),
            file_type: IncludeType::Toml,
            ref_name: "cargo".into(),
        })];
        let inc_ref = IncludeRef {
            include_ref: "cargo".into(),
            run: "scripts.build".into(),
            args: None,
            env: BTreeMap::new(),
            name: None,
        };
        let cmd = resolve_include_entry(&inc_ref, &includes).unwrap();
        assert_eq!(cmd.run, "cargo build --release");
    }

    #[test]
    fn test_resolve_include_local_yaml() {
        use std::io::Write;
        use tempfile::NamedTempFile;

        let mut f = NamedTempFile::new().unwrap();
        write!(f, "scripts:\n  lint: \"eslint .\"\n").unwrap();

        let includes = vec![IncludeEntry::Local(LocalInclude {
            path: f.path().to_str().unwrap().to_string(),
            file_type: IncludeType::Yaml,
            ref_name: "scripts".into(),
        })];
        let inc_ref = IncludeRef {
            include_ref: "scripts".into(),
            run: "scripts.lint".into(),
            args: None,
            env: BTreeMap::new(),
            name: None,
        };
        let cmd = resolve_include_entry(&inc_ref, &includes).unwrap();
        assert_eq!(cmd.run, "eslint .");
    }

    #[test]
    fn test_resolve_include_missing_ref() {
        let includes: Vec<IncludeEntry> = vec![];
        let inc_ref = IncludeRef {
            include_ref: "pkg".into(),
            run: "scripts.lint".into(),
            args: None,
            env: BTreeMap::new(),
            name: None,
        };
        let result = resolve_include_entry(&inc_ref, &includes);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("pkg"));
    }

    #[test]
    fn test_resolve_include_missing_file() {
        let includes = vec![IncludeEntry::Local(LocalInclude {
            path: "/nonexistent/path/file.json".into(),
            file_type: IncludeType::Json,
            ref_name: "pkg".into(),
        })];
        let inc_ref = IncludeRef {
            include_ref: "pkg".into(),
            run: "scripts.lint".into(),
            args: None,
            env: BTreeMap::new(),
            name: None,
        };
        assert!(resolve_include_entry(&inc_ref, &includes).is_err());
    }

    #[test]
    fn test_resolve_include_name_defaults_to_last_segment() {
        use std::io::Write;
        use tempfile::NamedTempFile;

        let mut f = NamedTempFile::new().unwrap();
        write!(f, r#"{{"scripts": {{"mytest": "jest --coverage"}}}}"#).unwrap();

        let includes = vec![IncludeEntry::Local(LocalInclude {
            path: f.path().to_str().unwrap().to_string(),
            file_type: IncludeType::Json,
            ref_name: "pkg".into(),
        })];
        let inc_ref = IncludeRef {
            include_ref: "pkg".into(),
            run: "scripts.mytest".into(),
            args: None,
            env: BTreeMap::new(),
            name: None,
        };
        let cmd = resolve_include_entry(&inc_ref, &includes).unwrap();
        assert_eq!(cmd.name, "mytest");
    }

    // -----------------------------------------------------------------------
    // validate_depends tests
    // -----------------------------------------------------------------------

    #[test]
    fn test_validate_depends_valid() {
        let cmds = vec![
            Command {
                name: "a".into(),
                run: "echo a".into(),
                depends: vec![],
                env: BTreeMap::new(),
                test: false,
                cache: None,
            },
            Command {
                name: "b".into(),
                run: "echo b".into(),
                depends: vec!["a".into()],
                env: BTreeMap::new(),
                test: false,
                cache: None,
            },
        ];
        assert!(validate_depends_pub(&cmds).is_ok());
    }

    #[test]
    fn test_validate_depends_unknown_dep() {
        let cmds = vec![Command {
            name: "b".into(),
            run: "echo b".into(),
            depends: vec!["nonexistent".into()],
            env: BTreeMap::new(),
            test: false,
            cache: None,
        }];
        let result = validate_depends_pub(&cmds);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("nonexistent"));
    }

    #[test]
    fn test_validate_depends_self_reference() {
        let cmds = vec![Command {
            name: "a".into(),
            run: "echo a".into(),
            depends: vec!["a".into()],
            env: BTreeMap::new(),
            test: false,
            cache: None,
        }];
        let result = validate_depends_pub(&cmds);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("itself"));
    }

    #[test]
    fn test_validate_depends_empty() {
        assert!(validate_depends_pub(&[]).is_ok());
    }

    // -----------------------------------------------------------------------
    // Config serialization round-trip tests
    // -----------------------------------------------------------------------

    #[test]
    fn test_config_roundtrip_with_include() {
        let mut config = Config::default();
        config.include.push(IncludeEntry::Local(LocalInclude {
            path: "package.json".into(),
            file_type: IncludeType::Json,
            ref_name: "pkg".into(),
        }));
        let yaml = serde_yaml::to_string(&config).unwrap();
        assert!(yaml.contains("source: local"), "serialized yaml: {}", yaml);
        let parsed: Config = serde_yaml::from_str(&yaml).unwrap();
        assert_eq!(parsed.include.len(), 1);
        assert_eq!(parsed.include[0].ref_name(), "pkg");
    }

    #[test]
    fn test_remote_include_type_defaults_to_yaml() {
        let yaml = "include:\n  - source: remote\n    url: 'https://example.com/file.yaml'\n    ref: myfile\n";
        let config: Config = serde_yaml::from_str(yaml).unwrap();
        if let IncludeEntry::Remote(r) = &config.include[0] {
            assert!(matches!(r.file_type, IncludeType::Yaml));
        } else {
            panic!("Expected Remote variant");
        }
    }
}
