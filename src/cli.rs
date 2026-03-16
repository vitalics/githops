use clap::{CommandFactory, Parser, Subcommand, ValueEnum};
use clap_complete::Shell;

#[derive(Parser)]
#[command(
    name = "githops",
    version,
    about = "Git hooks manager with YAML configuration",
    long_about = None
)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Subcommand)]
pub enum Commands {
    /// Initialize githops in the current repository.
    /// Creates githops.yaml and .githops/githops.schema.json
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

#[derive(ValueEnum, Clone, Debug)]
pub enum MigrateSource {
    Husky,
    Lefthook,
}

pub fn generate_completion(shell: Shell) {
    clap_complete::generate(shell, &mut Cli::command(), "githops", &mut std::io::stdout());
}
