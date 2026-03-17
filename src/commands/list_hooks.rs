use anyhow::Result;
use std::path::Path;

pub fn run() -> Result<()> {
    let path = Path::new(githops_core::config::CONFIG_FILE);
    let config = match githops_core::config::Config::load(path) {
        Ok(c) => c,
        Err(_) => return Ok(()),
    };
    for hook_info in githops_core::hooks::ALL_HOOKS {
        if config.hooks.get(hook_info.name).is_some() {
            println!("{}", hook_info.name);
        }
    }
    Ok(())
}
