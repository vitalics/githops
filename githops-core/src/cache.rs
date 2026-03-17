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

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_compute_key_is_deterministic() {
        let inputs = vec![(
            std::path::PathBuf::from("src/main.rs"),
            b"fn main() {}".to_vec(),
        )];
        let key1 = compute_key("cargo build", &[], &inputs);
        let key2 = compute_key("cargo build", &[], &inputs);
        assert_eq!(key1, key2);
    }

    #[test]
    fn test_compute_key_differs_on_run_change() {
        let inputs: Vec<(std::path::PathBuf, Vec<u8>)> = vec![];
        let key1 = compute_key("cargo build", &[], &inputs);
        let key2 = compute_key("cargo test", &[], &inputs);
        assert_ne!(key1, key2);
    }

    #[test]
    fn test_compute_key_differs_on_file_content_change() {
        let inputs1 = vec![(
            std::path::PathBuf::from("src/main.rs"),
            b"fn main() { println!(\"v1\"); }".to_vec(),
        )];
        let inputs2 = vec![(
            std::path::PathBuf::from("src/main.rs"),
            b"fn main() { println!(\"v2\"); }".to_vec(),
        )];
        let key1 = compute_key("build", &[], &inputs1);
        let key2 = compute_key("build", &[], &inputs2);
        assert_ne!(key1, key2);
    }

    #[test]
    fn test_compute_key_differs_on_extra_keys() {
        let inputs: Vec<(std::path::PathBuf, Vec<u8>)> = vec![];
        let key1 = compute_key("build", &["1.70.0".to_string()], &inputs);
        let key2 = compute_key("build", &["1.71.0".to_string()], &inputs);
        assert_ne!(key1, key2);
    }

    #[test]
    fn test_compute_key_is_hex_string() {
        let inputs: Vec<(std::path::PathBuf, Vec<u8>)> = vec![];
        let key = compute_key("build", &[], &inputs);
        assert_eq!(key.len(), 64); // SHA-256 → 32 bytes → 64 hex chars
        assert!(key.chars().all(|c| c.is_ascii_hexdigit()));
    }

    #[test]
    fn test_is_hit_false_before_recording() {
        let dir = TempDir::new().unwrap();
        let key = "abc123";
        assert!(!is_hit(key, dir.path()));
    }

    #[test]
    fn test_record_hit_and_then_is_hit() {
        let dir = TempDir::new().unwrap();
        let key = "deadbeef";
        assert!(!is_hit(key, dir.path()));
        record_hit(key, dir.path()).unwrap();
        assert!(is_hit(key, dir.path()));
    }

    #[test]
    fn test_record_hit_creates_ok_file() {
        let dir = TempDir::new().unwrap();
        record_hit("myhash", dir.path()).unwrap();
        assert!(dir.path().join("myhash.ok").exists());
    }

    #[test]
    fn test_is_hit_different_keys_independent() {
        let dir = TempDir::new().unwrap();
        record_hit("key1", dir.path()).unwrap();
        assert!(is_hit("key1", dir.path()));
        assert!(!is_hit("key2", dir.path()));
    }

    #[test]
    fn test_expand_globs_finds_existing_files() {
        let dir = TempDir::new().unwrap();
        std::fs::write(dir.path().join("a.rs"), "fn a() {}").unwrap();
        std::fs::write(dir.path().join("b.rs"), "fn b() {}").unwrap();
        let pattern = format!("{}/*.rs", dir.path().to_str().unwrap());
        let found = expand_globs(&[pattern], dir.path());
        assert_eq!(found.len(), 2);
    }

    #[test]
    fn test_expand_globs_empty_on_no_match() {
        let dir = TempDir::new().unwrap();
        let pattern = format!("{}/*.nonexistent", dir.path().to_str().unwrap());
        let found = expand_globs(&[pattern], dir.path());
        assert!(found.is_empty());
    }

    #[test]
    fn test_expand_globs_deduplicates() {
        let dir = TempDir::new().unwrap();
        std::fs::write(dir.path().join("main.rs"), "").unwrap();
        let path_str = dir.path().to_str().unwrap();
        // Same glob twice
        let patterns = vec![
            format!("{}/*.rs", path_str),
            format!("{}/*.rs", path_str),
        ];
        let found = expand_globs(&patterns, dir.path());
        assert_eq!(found.len(), 1);
    }

    #[test]
    fn test_read_inputs_reads_content() {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("test.rs");
        std::fs::write(&path, "hello world").unwrap();
        let result = read_inputs(&[path.clone()]);
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].1, b"hello world");
    }

    #[test]
    fn test_read_inputs_skips_missing_files() {
        let missing = std::path::PathBuf::from("/nonexistent/path/file.rs");
        let result = read_inputs(&[missing]);
        assert!(result.is_empty());
    }
}
