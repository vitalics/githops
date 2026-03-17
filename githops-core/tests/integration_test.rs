//! Integration tests for githops-core.
//! These tests exercise the full config load/resolve/sync pipeline
//! using real temporary files on disk.

use std::collections::BTreeMap;
use std::io::Write;
use tempfile::{NamedTempFile, TempDir};

use githops_core::config::{
    Command, CommandEntry, Config, HookConfig, IncludeEntry, IncludeRef, IncludeType, LocalInclude,
    resolve_include_entry,
};
use githops_core::sync_hooks::sync_to_hooks;

/// Write a config to a temp file and load it back.
fn write_and_load(yaml: &str) -> Config {
    let mut f = NamedTempFile::new().unwrap();
    write!(f, "{}", yaml).unwrap();
    Config::load(f.path()).unwrap()
}

#[test]
fn test_load_config_from_file() {
    let config = write_and_load(
        r#"
hooks:
  pre-commit:
    enabled: true
    commands:
      - name: lint
        run: echo lint
"#,
    );
    assert!(config.hooks.pre_commit.is_some());
}

#[test]
fn test_load_config_with_yaml_anchor() {
    // YAML anchors should be resolved transparently
    let config = write_and_load(
        r#"
x-env: &common-env
  CI: "true"

hooks:
  pre-commit:
    enabled: true
    commands:
      - name: lint
        run: echo lint
        env: *common-env
"#,
    );
    let hook = config.hooks.pre_commit.as_ref().unwrap();
    if let CommandEntry::Inline(cmd) = &hook.commands[0] {
        assert_eq!(cmd.env.get("CI").map(|s| s.as_str()), Some("true"));
    }
}

#[test]
fn test_load_config_with_definitions_and_ref() {
    let config = write_and_load(
        r#"
definitions:
  lint:
    name: lint
    run: cargo clippy

hooks:
  pre-commit:
    commands:
      - $ref: lint
"#,
    );
    let hook = config.hooks.pre_commit.as_ref().unwrap();
    let resolved = hook.resolved_commands(&config.definitions);
    assert_eq!(resolved.len(), 1);
    assert_eq!(resolved[0].run, "cargo clippy");
}

#[test]
fn test_full_sync_and_verify_marker() {
    let dir = TempDir::new().unwrap();
    let config = write_and_load(
        r#"
hooks:
  pre-commit:
    enabled: true
    commands:
      - name: lint
        run: echo lint
  commit-msg:
    enabled: true
    commands:
      - name: validate
        run: echo validate
"#,
    );

    let (installed, _) = sync_to_hooks(&config, dir.path(), false).unwrap();
    assert_eq!(installed, 2);

    let pre_commit = std::fs::read_to_string(dir.path().join("pre-commit")).unwrap();
    assert!(pre_commit.contains("GITHOPS_MANAGED"));
    assert!(pre_commit.contains("pre-commit"));

    let commit_msg = std::fs::read_to_string(dir.path().join("commit-msg")).unwrap();
    assert!(commit_msg.contains("GITHOPS_MANAGED"));
}

#[test]
fn test_include_resolve_from_json_file_integration() {
    let mut json_file = NamedTempFile::new().unwrap();
    write!(
        json_file,
        r#"{{"scripts": {{"lint": "eslint . --ext .ts", "test": "jest --coverage"}}}}"#
    )
    .unwrap();

    let includes = vec![IncludeEntry::Local(LocalInclude {
        path: json_file.path().to_str().unwrap().to_string(),
        file_type: IncludeType::Json,
        ref_name: "pkg".into(),
    })];

    let lint_ref = IncludeRef {
        include_ref: "pkg".into(),
        run: "scripts.lint".into(),
        args: None,
        env: Default::default(),
        name: None,
    };
    let test_ref = IncludeRef {
        include_ref: "pkg".into(),
        run: "scripts.test".into(),
        args: None,
        env: Default::default(),
        name: None,
    };

    let lint_cmd = resolve_include_entry(&lint_ref, &includes).unwrap();
    let test_cmd = resolve_include_entry(&test_ref, &includes).unwrap();

    assert_eq!(lint_cmd.run, "eslint . --ext .ts");
    assert_eq!(lint_cmd.name, "lint");
    assert_eq!(test_cmd.run, "jest --coverage");
    assert_eq!(test_cmd.name, "test");
}

#[test]
fn test_include_resolve_from_toml_file_integration() {
    let mut toml_file = NamedTempFile::new().unwrap();
    write!(
        toml_file,
        "[scripts]\nbuild = \"cargo build --release\"\ntest = \"cargo test\"\n"
    )
    .unwrap();

    let includes = vec![IncludeEntry::Local(LocalInclude {
        path: toml_file.path().to_str().unwrap().to_string(),
        file_type: IncludeType::Toml,
        ref_name: "cargo".into(),
    })];

    let ref_ = IncludeRef {
        include_ref: "cargo".into(),
        run: "scripts.build".into(),
        args: None,
        env: Default::default(),
        name: None,
    };
    let cmd = resolve_include_entry(&ref_, &includes).unwrap();
    assert_eq!(cmd.run, "cargo build --release");
}

#[test]
fn test_include_resolve_from_yaml_file_integration() {
    let mut yaml_file = NamedTempFile::new().unwrap();
    write!(
        yaml_file,
        "scripts:\n  format: \"rustfmt --check src/**/*.rs\"\n"
    )
    .unwrap();

    let includes = vec![IncludeEntry::Local(LocalInclude {
        path: yaml_file.path().to_str().unwrap().to_string(),
        file_type: IncludeType::Yaml,
        ref_name: "scripts".into(),
    })];

    let ref_ = IncludeRef {
        include_ref: "scripts".into(),
        run: "scripts.format".into(),
        args: None,
        env: Default::default(),
        name: None,
    };
    let cmd = resolve_include_entry(&ref_, &includes).unwrap();
    assert_eq!(cmd.run, "rustfmt --check src/**/*.rs");
}

#[test]
fn test_config_save_and_reload_roundtrip() {
    let dir = TempDir::new().unwrap();
    let path = dir.path().join("githops.yaml");

    let mut config = Config::default();
    config.hooks.pre_commit = Some(HookConfig {
        enabled: true,
        parallel: true,
        commands: vec![CommandEntry::Inline(Command {
            name: "lint".into(),
            run: "echo lint".into(),
            depends: vec![],
            env: BTreeMap::new(),
            test: false,
            cache: None,
        })],
    });
    config.save(&path).unwrap();

    let loaded = Config::load(&path).unwrap();
    let hook = loaded.hooks.pre_commit.as_ref().unwrap();
    assert!(hook.enabled);
    assert!(hook.parallel);
    assert_eq!(hook.commands.len(), 1);
}

/// End-to-end scenario: a githops.yaml with `source: local` include pointing to a
/// package.json on disk.  The config is loaded, the include is resolved, and the
/// resolved command is synced into a hook script.
#[test]
fn test_source_local_include_end_to_end() {
    // 1. Write a package.json with a test script
    let mut pkg = NamedTempFile::new().unwrap();
    write!(
        pkg,
        r#"{{"scripts": {{"test": "jest --coverage", "lint": "eslint ."}}}}"#
    )
    .unwrap();
    let pkg_path = pkg.path().to_str().unwrap();

    // 2. Write githops.yaml using the `source: local` flat format
    let yaml = format!(
        r#"
include:
  - source: local
    path: {pkg_path}
    type: json
    ref: packagejson

hooks:
  pre-commit:
    enabled: true
    parallel: false
    commands:
      - name: fmt
        run: echo "Run your formatter"
      - name: lint
        run: echo "Run your linter"
      - $include: packagejson
        run: scripts.test
"#
    );

    let config = write_and_load(&yaml);

    // 3. Verify include was parsed correctly
    assert_eq!(config.include.len(), 1);
    assert!(matches!(config.include[0], IncludeEntry::Local(_)));
    assert_eq!(config.include[0].ref_name(), "packagejson");

    // 4. Resolve commands including the include
    let hook = config.hooks.pre_commit.as_ref().unwrap();
    let commands = hook
        .resolved_commands_with_includes(&config.definitions, &config.include)
        .unwrap();

    assert_eq!(commands.len(), 3);
    assert_eq!(commands[0].run, "echo \"Run your formatter\"");
    assert_eq!(commands[1].run, "echo \"Run your linter\"");
    assert_eq!(commands[2].run, "jest --coverage");
    assert_eq!(commands[2].name, "test");

    // 5. Sync to a temp hooks dir and verify the generated script delegates to githops check
    let hooks_dir = TempDir::new().unwrap();
    let (installed, _) = sync_to_hooks(&config, hooks_dir.path(), false).unwrap();
    assert_eq!(installed, 1);

    let script = std::fs::read_to_string(hooks_dir.path().join("pre-commit")).unwrap();
    assert!(script.contains("GITHOPS_MANAGED"));
    assert!(script.contains("githops check pre-commit"));
}

#[test]
fn test_sync_multiple_hooks_then_remove_one() {
    let dir = TempDir::new().unwrap();

    // First sync: two hooks
    let config = write_and_load(
        r#"
hooks:
  pre-commit:
    enabled: true
    commands:
      - name: lint
        run: echo lint
  pre-push:
    enabled: true
    commands:
      - name: test
        run: echo test
"#,
    );
    sync_to_hooks(&config, dir.path(), false).unwrap();
    assert!(dir.path().join("pre-commit").exists());
    assert!(dir.path().join("pre-push").exists());

    // Second sync: only one hook remains
    let config2 = write_and_load(
        r#"
hooks:
  pre-commit:
    enabled: true
    commands:
      - name: lint
        run: echo lint
"#,
    );
    sync_to_hooks(&config2, dir.path(), false).unwrap();
    assert!(dir.path().join("pre-commit").exists());
    // pre-push was managed and is now unconfigured → should be removed
    assert!(!dir.path().join("pre-push").exists());
}
