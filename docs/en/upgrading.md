# Upgrading

## Self-update

The easiest way to update githops is the built-in self-update command:

```sh
githops self-update
```

This downloads the latest release for your platform from GitHub, replaces the current binary, and prints the new version.

To check if an update is available without installing it:

```sh
githops self-update --check
```

## Upgrading via package managers

### Cargo

```sh
cargo install githops --force
```

### npm / pnpm

```sh
# npm
npm install --save-dev githops@latest

# pnpm
pnpm add --save-dev githops@latest
```

### macOS PKG / DMG

Download the latest `.pkg` installer from the [releases page](https://github.com/vitalics/githops/releases/latest) and run it. The installer detects an existing installation via the `UpgradeCode` embedded in the package and upgrades in place.

### Windows MSI

Download the latest `.msi` from the [releases page](https://github.com/vitalics/githops/releases/latest) and run it. The MSI installer handles the upgrade automatically.

## Configuration compatibility

githops follows semantic versioning. Minor and patch releases are backwards-compatible — your existing `githops.yaml` will continue to work after upgrading.

Major version bumps may introduce breaking changes. When a major version is released, a migration note is published in the release changelog describing what changed and how to update your configuration.

## Regenerating the JSON Schema

After upgrading, regenerate the schema file to get IntelliSense for any new fields:

```sh
githops init
```

This only writes `.githops/githops.schema.json` — it does not modify `githops.yaml`.

## Re-syncing hooks after upgrade

The hook scripts written into `.git/hooks/` by `githops sync` delegate to the `githops` binary. In most cases you do not need to re-sync after upgrading because the scripts just call `githops run <hook-name>` and use whatever binary is on `PATH`.

If a major version changes the hook runner interface, the release notes will say so and include instructions for re-syncing.

## Pinning a version

If you need a specific version of githops in a CI environment or want to avoid automatic updates:

```sh
# Install a specific version via cargo
cargo install githops --version 1.2.3 --force

# Or via npm
npm install --save-dev githops@1.2.3
```

## Changelog

The full changelog is available on the [GitHub releases page](https://github.com/vitalics/githops/releases). Each release includes a list of commits and installation instructions.
