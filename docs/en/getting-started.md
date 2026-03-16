# Getting Started

## Installation

### macOS

The recommended way to install githops on macOS is through the `.pkg` installer, which places the binary in `/usr/local/bin` automatically.

```sh
# Apple Silicon (M1 and later)
curl -fsSL https://github.com/vitalics/githops/releases/latest/download/githops-latest-aarch64-apple-darwin.pkg -o githops.pkg
sudo installer -pkg githops.pkg -target /

# Intel
curl -fsSL https://github.com/vitalics/githops/releases/latest/download/githops-latest-x86_64-apple-darwin.pkg -o githops.pkg
sudo installer -pkg githops.pkg -target /
```

### Linux

```sh
# x86_64
curl -fsSL https://github.com/vitalics/githops/releases/latest/download/githops-latest-x86_64-unknown-linux-gnu.tar.gz | tar xz
sudo mv githops /usr/local/bin/

# ARM64
curl -fsSL https://github.com/vitalics/githops/releases/latest/download/githops-latest-aarch64-unknown-linux-gnu.tar.gz | tar xz
sudo mv githops /usr/local/bin/
```

### Windows

Download and run the MSI installer from the [releases page](https://github.com/vitalics/githops/releases/latest). It installs `githops.exe` and adds it to the system `PATH` automatically.

### Cargo

If you have the Rust toolchain installed:

```sh
cargo install githops
```

### npm / pnpm

githops will be published to npm for use in JavaScript projects:

```sh
# npm
npm install --save-dev githops

# pnpm
pnpm add --save-dev githops
```

After installing via npm/pnpm, add a `prepare` script to your `package.json` so hooks are installed automatically when team members run `npm install`:

```json
{
  "scripts": {
    "prepare": "githops sync"
  }
}
```

## Verify installation

```sh
githops --version
```

## Initialize a repository

Navigate to the root of any Git repository and run:

```sh
githops init
```

This creates `githops.yaml` with a commented example, and writes a `.githops/githops.schema.json` file that enables YAML IntelliSense in VS Code and other editors that support the `yaml-language-server` protocol.

## Your first hook

Open `githops.yaml` and add a `pre-commit` hook:

```yaml
# yaml-language-server: $schema=.githops/githops.schema.json

hooks:
  pre-commit:
    enabled: true
    commands:
      - name: lint
        run: npm run lint
      - name: typecheck
        run: npm run typecheck
        depends:
          - lint
```

The `depends` field tells githops to run `typecheck` only after `lint` succeeds. Commands without dependencies run first; commands with dependencies wait for them.

## Install the hooks

```sh
githops sync
```

This writes the actual hook scripts into `.git/hooks/`. You need to run `sync` after changing `githops.yaml`. If you add `githops sync` to your `prepare` script (see npm section above), this happens automatically.

## Test the hook

Make a change to a tracked file and commit:

```sh
git add .
git commit -m "test"
```

githops runs your `pre-commit` hook. If any command exits with a non-zero code, the commit is aborted.

## Check for updates

```sh
githops self-update --check
```

To install the latest version:

```sh
githops self-update
```

## Shell completions

Install shell completions for your current shell:

```sh
githops completions init
```

This writes a completion script and patches your shell's rc file (`~/.zshrc` or `~/.bashrc`). Restart your terminal or source the rc file to activate completions.

## Next steps

- Read the [Features](./yaml-schema) section to learn about YAML Schema, templates, parallelization, the graph UI, and caching.
- If you are moving from Husky, lefthook, or pre-commit, see the [Migration Guide](./migration).
