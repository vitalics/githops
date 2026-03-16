use std::path::PathBuf;
use std::process::Command;

fn main() {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let ui_dir = manifest_dir.join("ui");
    let dist_dir = ui_dir.join("dist");

    // Rerun if any UI source changes.
    println!("cargo:rerun-if-changed=ui/index.html");
    println!("cargo:rerun-if-changed=ui/vite.config.ts");
    println!("cargo:rerun-if-changed=ui/package.json");
    println!("cargo:rerun-if-changed=ui/src");

    // When published to crates.io only ui/dist/ is included, not the UI sources.
    // Detect this by the absence of ui/package.json and use the pre-built dist/.
    if !ui_dir.join("package.json").exists() {
        assert!(
            dist_dir.join("index.html").exists(),
            "ui/package.json not found and ui/dist/index.html is missing — \
             the published crate is missing pre-built assets"
        );
        println!("cargo:warning=ui/package.json not found; using pre-built ui/dist/");
        return;
    }

    let pnpm = if cfg!(windows) { "pnpm.cmd" } else { "pnpm" };

    // Check if pnpm is available.
    if Command::new(pnpm).arg("--version").output().is_err() {
        if dist_dir.exists() {
            println!("cargo:warning=pnpm not found; using pre-built graphui/ui/dist/");
            return;
        }
        panic!(
            "pnpm is not available and graphui/ui/dist/ does not exist.\n\
             Install pnpm (https://pnpm.io/installation) or run `pnpm install && pnpm build` \
             inside graphui/ui/ manually."
        );
    }

    // Install dependencies when node_modules is missing.
    if !ui_dir.join("node_modules").exists() {
        let status = Command::new(pnpm)
            .arg("install")
            .current_dir(&ui_dir)
            .status()
            .expect("failed to spawn pnpm install");
        assert!(status.success(), "pnpm install failed in graphui/ui/");
    }

    // Build.
    let status = Command::new(pnpm)
        .args(["run", "build"])
        .current_dir(&ui_dir)
        .status()
        .expect("failed to spawn pnpm run build");
    assert!(status.success(), "pnpm run build failed in graphui/ui/");

    assert!(
        dist_dir.join("index.html").exists(),
        "graphui/ui/dist/index.html not found after vite build"
    );
}
