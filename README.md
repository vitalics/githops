# githops

Git hooks manager with YAML configuration. Declare your hooks once, commit the file, sync once — everyone on the team gets the same hooks automatically.

[![License: MIT](https://img.shields.io/badge/License-MIT-blue.svg)](LICENSE)
[![Crates.io](https://img.shields.io/crates/v/githops.svg)](https://crates.io/crates/githops)

---

## Why

Git hooks live in `.git/hooks/`, which is not committed to the repository. New team members miss them. The same linting command gets copy-pasted into three different hooks. There is no single place to see what runs and why.

githops solves this with a single `githops.yaml` file that you commit. It supports reusable command definitions, parallel execution, dependency ordering, content-based caching, and a browser-based visual graph of your entire hook configuration.

## Installation

**macOS**

```sh
# Apple Silicon
curl -fsSL https://github.com/vitalics/githops/releases/latest/download/githops-latest-aarch64-apple-darwin.pkg -o githops.pkg
sudo installer -pkg githops.pkg -target /

# Intel
curl -fsSL https://github.com/vitalics/githops/releases/latest/download/githops-latest-x86_64-apple-darwin.pkg -o githops.pkg
sudo installer -pkg githops.pkg -target /
```

**Linux**

```sh
curl -fsSL https://github.com/vitalics/githops/releases/latest/download/githops-latest-x86_64-unknown-linux-gnu.tar.gz | tar xz
sudo mv githops /usr/local/bin/
```

**Windows** — download and run the `.msi` installer from the [releases page](https://github.com/vitalics/githops/releases/latest).

**Cargo**

```sh
cargo install githops
```

**npm / pnpm**

```sh
npm install --save-dev githops
# or
pnpm add --save-dev githops
```

## Quick start

```sh
# 1. Initialize in any git repository
githops init

# 2. Edit githops.yaml
# 3. Install the hooks
githops sync
```

Example `githops.yaml`:

```yaml
# yaml-language-server: $schema=.githops/githops.schema.json

hooks:
  pre-commit:
    enabled: true
    parallel: true
    commands:
      - name: lint
        run: npm run lint
      - name: typecheck
        run: npx tsc --noEmit
        cache:
          inputs:
            - "src/**/*.ts"
          key:
            - tsconfig.json
```

## Features

- **Single YAML file** — all hooks in one place, committed to the repository
- **Reusable definitions** — extract common commands with `$ref`, no copy-pasting
- **YAML anchors** — standard YAML `&anchor` / `*alias` for sharing config fragments
- **Parallel execution** — `parallel: true` runs commands concurrently within a hook
- **Dependency ordering** — `depends` ensures commands run in the right sequence
- **Content-based caching** — skip commands when their input files have not changed
- **Visual graph UI** — `githops graph --open` shows and edits the full dependency graph in a browser
- **Shell completions** — `githops completions init` for bash and zsh
- **Self-update** — `githops self-update` upgrades the binary in place
- **No runtime dependency** — single Rust binary, works on macOS, Linux, and Windows

## Commands

| Command | Description |
|---|---|
| `githops init` | Create `githops.yaml` and write the JSON Schema |
| `githops sync` | Install configured hooks into `.git/hooks/` |
| `githops sync --force` | Overwrite hooks not managed by githops |
| `githops graph` | Start the visual graph UI (add `--open` to open browser) |
| `githops check` | Run all configured hooks without a git event |
| `githops migrate` | Import configuration from Husky, lefthook, or pre-commit |
| `githops completions init` | Install shell completions for the current shell |
| `githops self-update` | Download and install the latest release |
| `githops self-update --check` | Check if an update is available |

## Documentation

Full documentation is available at `/docs` when running `githops graph`, or in the [`docs/`](docs/) directory:

- [Introduction](docs/en/intro.md)
- [githops vs Husky / lefthook / pre-commit](docs/en/comparison.md)
- [Getting Started](docs/en/getting-started.md)
- Features: [YAML Schema](docs/en/features-yaml-schema.md) · [Templates](docs/en/features-templates.md) · [Parallelization](docs/en/features-parallelization.md) · [Graph UI](docs/en/features-graph-ui.md) · [Caching](docs/en/features-caching.md)
- [Migration Guide](docs/en/migration.md)
- [Upgrading](docs/en/upgrading.md)

Russian documentation is available in [`docs/ru/`](docs/ru/).

## Repository structure

```
githops/
├── src/                  # CLI binary (clap commands)
├── githops-core/         # Core library (config, sync, schema)
├── graphui/              # Visual graph UI (Axum + React/Vite)
│   └── ui/               # Frontend (TypeScript, Tailwind, React Router)
├── docs/                 # Documentation source
│   ├── en/               # English
│   └── ru/               # Russian
├── scripts/              # Release scripts
└── wix/                  # Windows installer (WiX v3)
```

## License

MIT — see [LICENSE](LICENSE).
