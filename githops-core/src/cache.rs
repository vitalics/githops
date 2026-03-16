use anyhow::Result;
use sha2::{Digest, Sha256};
use std::path::{Path, PathBuf};

// ---------------------------------------------------------------------------
// Input discovery
// ---------------------------------------------------------------------------

/// Expand glob patterns relative to `root` and return sorted, deduplicated
/// file paths.  Directories and broken globs are silently skipped.
pub fn expand_globs(patterns: &[String], root: &Path) -> Vec<PathBuf> {
    let mut paths = Vec::new();
    for pattern in patterns {
        let full = root.join(pattern).to_string_lossy().into_owned();
        if let Ok(entries) = glob::glob(&full) {
            for entry in entries.flatten() {
                if entry.is_file() {
                    paths.push(entry);
                }
            }
        }
    }
    paths.sort();
    paths.dedup();
    paths
}

/// Read the contents of each path.  Files that cannot be read are silently
/// omitted (the cache will miss on the next run, triggering a re-execution).
pub fn read_inputs(paths: &[PathBuf]) -> Vec<(PathBuf, Vec<u8>)> {
    paths
        .iter()
        .filter_map(|p| std::fs::read(p).ok().map(|c| (p.clone(), c)))
        .collect()
}

// ---------------------------------------------------------------------------
// Key computation
// ---------------------------------------------------------------------------

/// Compute a deterministic SHA-256 cache key from:
/// * the command's `run` script
/// * any extra `key` strings declared in the command cache config
/// * the path and content of every input file (in sorted path order)
pub fn compute_key(
    run: &str,
    extra_keys: &[String],
    input_files: &[(PathBuf, Vec<u8>)],
) -> String {
    let mut hasher = Sha256::new();

    hasher.update(run.as_bytes());

    for k in extra_keys {
        hasher.update(b"\x00key\x00");
        hasher.update(k.as_bytes());
    }

    for (path, content) in input_files {
        hasher.update(b"\x00file\x00");
        hasher.update(path.to_string_lossy().as_bytes());
        hasher.update(b"\x00");
        hasher.update(content);
    }

    format!("{:x}", hasher.finalize())
}

// ---------------------------------------------------------------------------
// Cache store (marker-file strategy)
// ---------------------------------------------------------------------------

/// Returns `true` when a cache entry exists for `key`, meaning the command's
/// last run with these exact inputs succeeded.
pub fn is_hit(key: &str, cache_dir: &Path) -> bool {
    cache_dir.join(format!("{}.ok", key)).exists()
}

/// Persist a successful run in the cache so future invocations can skip it.
pub fn record_hit(key: &str, cache_dir: &Path) -> Result<()> {
    std::fs::create_dir_all(cache_dir)?;
    // Write a marker file whose name encodes the key.
    std::fs::write(cache_dir.join(format!("{}.ok", key)), b"")?;
    Ok(())
}
