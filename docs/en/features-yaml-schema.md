# YAML Schema

githops ships a JSON Schema for `githops.yaml`. When you run `githops init`, the schema file is written to `.githops/githops.schema.json` and the configuration file gets a header comment that points editors to it.

## Editor integration

`githops.yaml` starts with:

```yaml
# yaml-language-server: $schema=.githops/githops.schema.json
```

This comment is recognized by `yaml-language-server`, which powers YAML IntelliSense in VS Code (via the YAML extension by Red Hat), Neovim (via `nvim-lspconfig`), and IntelliJ IDEA. You get:

- Autocompletion for all keys (`hooks`, `definitions`, `cache`, hook names, command fields).
- Inline documentation for every field on hover.
- Validation errors for unknown fields or wrong types, shown directly in the editor.

## Configuration reference

### Top-level structure

```yaml
# yaml-language-server: $schema=.githops/githops.schema.json

hooks:
  <hook-name>:
    enabled: true
    parallel: false
    commands: []

definitions:
  <definition-name>:
    name: string
    run: string

cache:
  enabled: false
  dir: .githops/cache
```

### hooks

A map of Git hook names to hook configurations. Supported hook names are all standard Git hooks: `pre-commit`, `prepare-commit-msg`, `commit-msg`, `post-commit`, `pre-push`, `pre-rebase`, `post-merge`, `post-checkout`, `post-rewrite`, `pre-merge-commit`, `pre-receive`, `update`, `post-receive`, `post-update`, and others.

#### Hook fields

| Field | Type | Default | Description |
|---|---|---|---|
| `enabled` | boolean | `true` | Whether the hook is active. Set to `false` to disable without deleting the config. |
| `parallel` | boolean | `false` | Run all commands in this hook concurrently. Commands with `depends` still wait for their dependencies. |
| `commands` | array | `[]` | Ordered list of commands or definition references. |

#### Command fields (inline)

| Field | Type | Default | Description |
|---|---|---|---|
| `name` | string | required | Unique name within this hook. Used in `depends` references. |
| `run` | string | required | Shell command to execute. Runs in a `sh -c` subshell on Unix, `cmd /c` on Windows. |
| `depends` | string[] | `[]` | Names of commands in the same hook that must complete successfully before this command runs. |
| `env` | object | `{}` | Environment variables to set for this command only. |
| `test` | boolean | `false` | If `true`, the command is only run when githops is invoked in test mode (e.g. `githops check --test`). |
| `cache` | object | — | Enable content-based caching for this command. See [Caching](./caching). |

#### Command reference (`$ref`)

Instead of an inline command, you can reference a definition:

```yaml
commands:
  - $ref: my-definition
    args: "--fix"
    name: "lint with fix"   # optional name override
```

| Field | Type | Description |
|---|---|---|
| `$ref` | string | Name of the definition to use. |
| `args` | string | Extra arguments appended to the definition's `run` command. |
| `name` | string | Override the display name for this use of the definition. |

### definitions

A map of reusable command definitions. Each definition can be a single command or a list of commands.

**Single command:**

```yaml
definitions:
  lint:
    name: Run ESLint
    run: npx eslint .
    depends: []
    env: {}
    test: false
```

**Command list:**

```yaml
definitions:
  setup:
    - name: install
      run: npm ci
    - name: build
      run: npm run build
      depends:
        - install
```

### cache

Global cache settings.

| Field | Type | Default | Description |
|---|---|---|---|
| `enabled` | boolean | `false` | Enable content-based caching globally. |
| `dir` | string | `.githops/cache` | Directory where cache marker files are stored. |

## Regenerating the schema

If you update githops and want to refresh the schema file:

```sh
githops init
```

`init` is safe to run on an existing project — it only writes the schema file and does not overwrite `githops.yaml`.
