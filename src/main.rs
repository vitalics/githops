use anyhow::Result;
use clap::Parser;
use githops::cli::{generate_completion, CacheAction, Cli, Commands, CompletionsAction, SchemaAction};
use githops::commands;

fn main() -> Result<()> {
    let cli = Cli::parse();
    match cli.command {
        Commands::Init => commands::init::run(),
        Commands::Sync { force } => commands::sync::run(force),
        Commands::Graph { open } => graphui::run(open),
        Commands::Destroy => commands::destroy::run(),
        Commands::Migrate { source } => commands::migrate::run(source),
        Commands::Check { hook, args } => commands::check::run(&hook, &args),
        Commands::SelfUpdate { check } => commands::self_update::run(check),
        Commands::Completions { action } => match action {
            CompletionsAction::Init => commands::completions::init(),
            CompletionsAction::Remove => commands::completions::remove(),
            CompletionsAction::Print { shell } => {
                generate_completion(shell);
                Ok(())
            }
        },
        Commands::Schema { action } => match action {
            SchemaAction::Sync => commands::schema::sync(),
        },
        Commands::Cache { action } => match action {
            CacheAction::Clear => commands::cache::clear(),
            CacheAction::List => commands::cache::list(),
        },
    }
}
