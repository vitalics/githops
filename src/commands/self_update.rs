use anyhow::Result;
use colored::Colorize;

const REPO_OWNER: &str = "vitaliharadkou";
const REPO_NAME: &str = "githops";

pub fn run(check_only: bool) -> Result<()> {
    let current = self_update::cargo_crate_version!();
    let target = self_update::get_target();

    println!(
        "{} {} › {} ({})",
        "githops".cyan().bold(),
        "self-update".cyan(),
        current,
        target
    );

    // Fetch the latest release to compare versions before downloading.
    let releases = self_update::backends::github::ReleaseList::configure()
        .repo_owner(REPO_OWNER)
        .repo_name(REPO_NAME)
        .build()?
        .fetch()?;

    let latest = match releases.first() {
        Some(r) => r,
        None => {
            println!("{} No releases found.", "warn:".yellow().bold());
            return Ok(());
        }
    };

    // self_update strips a leading 'v' when comparing.
    let latest_ver = latest.version.trim_start_matches('v');

    if self_update::version::bump_is_greater(current, latest_ver)? {
        if check_only {
            println!(
                "{} v{} is available (current: v{}).",
                "update:".green().bold(),
                latest_ver,
                current
            );
            println!(
                "  Run {} to install.",
                "githops self-update".cyan().bold()
            );
            return Ok(());
        }

        println!(
            "{} v{} → v{}…",
            "updating:".green().bold(),
            current,
            latest_ver
        );

        self_update::backends::github::Update::configure()
            .repo_owner(REPO_OWNER)
            .repo_name(REPO_NAME)
            .bin_name("githops")
            .target(target)
            .current_version(current)
            .show_download_progress(true)
            .build()?
            .update()?;

        println!("{} now at v{}.", "done:".green().bold(), latest_ver);
    } else {
        println!(
            "{} Already up to date (v{}).",
            "info:".cyan().bold(),
            current
        );
    }

    Ok(())
}
