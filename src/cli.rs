use clap::{CommandFactory, Parser, Subcommand, ValueEnum};
use clap_complete::Shell;

/// Default log template used when `--verbose-template` is not specified.
pub const DEFAULT_VERBOSE_TEMPLATE: &str = "[$t] [$k] ($l) $m";

#[derive(Parser)]
#[command(
    name = "githops",
    about = "Git hooks manager with YAML configuration",
    long_about = None,
    version,
)]
pub struct Cli {
    /// Enable verbose logging.
    /// Emits structured log lines to stderr for every operation.
    /// When omitted only INFO and ERROR entries are shown.
    #[arg(short = 'v', long, global = true)]
    pub verbose: bool,

    /// Custom log line template.
    ///
    /// Tokens: $t (time), $k (kind), $l (layer), $m (message).
    ///
    /// Default: "[$t] [$k] ($l) $m"
    ///
    /// Example: --verbose-template "$t $k: $m"
    #[arg(long, global = true, default_value = DEFAULT_VERBOSE_TEMPLATE)]
    pub verbose_template: String,

    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Subcommand)]
pub enum Commands {
    /// Initialize githops in the current repository.
    /// Creates githops.yaml, .githops/githops.schema.json, and syncs hooks.
    Init,

    /// Sync hooks from githops.yaml to .git/hooks/
    Sync {
        /// Overwrite hooks that are not managed by githops
        #[arg(long, short)]
        force: bool,
    },

    /// Start the hooks graph UI (local HTTP server).
    /// Use --open to launch the browser automatically.
    Graph {
        /// Open the browser immediately after starting the server
        #[arg(long, short)]
        open: bool,
    },

    /// Remove all githops-managed hooks from .git/hooks/
    Destroy,

    /// Migrate existing hooks from husky or lefthook to githops.yaml
    Migrate {
        /// Source tool to migrate from
        #[arg(value_enum, default_value = "husky")]
        source: MigrateSource,
    },

    /// Run commands for a specific hook stage (called by git hooks)
    Check {
        /// Hook stage name (e.g. pre-commit, commit-msg, pre-push)
        #[arg(value_parser = clap::builder::PossibleValuesParser::new(
            githops_core::hooks::ALL_HOOKS.iter().map(|h| h.name)
        ))]
        hook: String,

        /// Arguments forwarded from git (e.g. commit message file path)
        #[arg(trailing_var_arg = true, allow_hyphen_values = true)]
        args: Vec<String>,
    },

    /// Update githops to the latest release from GitHub
    SelfUpdate {
        /// Only check if an update is available without installing it
        #[arg(long, short)]
        check: bool,
    },

    /// Manage shell completion scripts
    Completions {
        #[command(subcommand)]
        action: CompletionsAction,
    },

    /// Manage the JSON Schema for githops.yaml
    Schema {
        #[command(subcommand)]
        action: SchemaAction,
    },

    /// Manage the build cache
    Cache {
        #[command(subcommand)]
        action: CacheAction,
    },

    /// Internal: print configured hook names from local githops.yaml (used by shell completions)
    #[command(name = "_list-hooks", hide = true)]
    ListHooks,
}

#[derive(Subcommand, Clone, Debug)]
pub enum CompletionsAction {
    /// Install completions for the current shell automatically
    Init,

    /// Remove all installed githops completion files
    Remove,

    /// Print completion script to stdout (for manual installation)
    ///
    /// Examples:
    ///   githops completions print bash >> ~/.local/share/bash-completion/completions/githops
    ///   githops completions print zsh > ~/.zfunc/_githops
    ///   githops completions print fish > ~/.config/fish/completions/githops.fish
    Print {
        /// Target shell
        shell: Shell,
    },
}

#[derive(Subcommand, Clone, Debug)]
pub enum SchemaAction {
    /// Update .githops/githops.schema.json to the version embedded in this binary
    Sync,
}

#[derive(Subcommand, Clone, Debug)]
pub enum CacheAction {
    /// Remove all cache entries
    Clear,

    /// List all cache entries with their key and age
    #[command(alias = "ls")]
    List,
}

#[derive(ValueEnum, Clone, Debug)]
pub enum MigrateSource {
    Husky,
    Lefthook,
}

pub fn generate_completion(shell: Shell) {
    let mut buf = Vec::new();
    clap_complete::generate(shell, &mut Cli::command(), "githops", &mut buf);
    print!("{}", patch_completion(shell, &buf));
}

/// Patch a generated completion script to use `githops _list-hooks` for dynamic
/// hook-name completions in `githops check <hook>`.
pub fn patch_completion(shell: Shell, buf: &[u8]) -> String {
    let script = String::from_utf8_lossy(buf).into_owned();
    match shell {
        Shell::Bash => {
            let override_block = concat!(
                "\n# Dynamic hook completion for `githops check <hook>`\n",
                "__githops_check_hook() {\n",
                "    local cur=\"${COMP_WORDS[COMP_CWORD]}\"\n",
                "    if [[ \"${COMP_WORDS[1]}\" == \"check\" && \"${COMP_CWORD}\" -eq 2 ]]; then\n",
                "        local IFS=$'\\n'\n",
                "        COMPREPLY=($(compgen -W \"$(githops _list-hooks 2>/dev/null)\" -- \"${cur}\"))\n",
                "        return 0\n",
                "    fi\n",
                "    _githops \"$@\"\n",
                "}\n",
                "complete -F __githops_check_hook githops\n",
            );
            format!("{script}{override_block}")
        }
        Shell::Zsh => {
            let helper = concat!(
                "\n# Dynamic hook completion for `githops check <hook>`\n",
                "__githops_hook_names() {\n",
                "  local -a hooks\n",
                "  hooks=(${(f)\"$(githops _list-hooks 2>/dev/null)\"})\n",
                "  (( ${#hooks[@]} )) && _describe 'git hook' hooks\n",
                "}\n",
            );
            // Replace the static possible-values spec with the dynamic function.
            // clap_complete 4 zsh generates ':hook -- description:(val1 val2 ...)' for
            // possible_values. Find the opening ' of the spec and the closing )' and
            // replace the whole token.
            let patched = if let Some(start) = script.find("':hook") {
                if let Some(rel_end) = script[start + 1..].find(")'") {
                    let end = start + 1 + rel_end + 2;
                    format!("{}':hook:__githops_hook_names'{}", &script[..start], &script[end..])
                } else {
                    script
                }
            } else {
                script
            };
            format!("{patched}{helper}")
        }
        Shell::Fish => {
            let extra = concat!(
                "\n# Dynamic hook completion for 'githops check <hook>'\n",
                "complete -c githops -n \"__fish_seen_subcommand_from check\" -e\n",
                "complete -c githops -n \"__fish_seen_subcommand_from check\" -f -a \"(githops _list-hooks 2>/dev/null)\"\n",
            );
            format!("{script}{extra}")
        }
        _ => script,
    }
}
