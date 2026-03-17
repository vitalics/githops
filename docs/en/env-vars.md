# Environment Variables

## Variables read by githops

| Variable | Used by | Description |
|----------|---------|-------------|
| `SHELL`  | `githops completions init` | Detects the current shell to choose the right completion format and install path. |
| `HOME`   | `githops completions` | Resolves `~` when writing completion files and patching RC files. |

## Variables set in hook commands

Any command entry in `githops.yaml` can define an `env:` map. githops injects these variables into the subprocess when running that command:

```yaml
hooks:
  pre-commit:
    commands:
      - name: lint
        run: eslint .
        env:
          NODE_ENV: production
          CI: "true"
```

These variables are scoped to the individual command and do not leak between commands or back into the githops process.

The `$include` command entry also supports `env:`:

```yaml
hooks:
  pre-commit:
    commands:
      - $include: packagejson
        run: scripts.lint
        args: "--fix"
        env:
          NODE_ENV: production
```

## Variables forwarded from git

When git invokes a hook, it passes arguments that are available to every command via `$1`, `$2`, etc. For `commit-msg` git passes the path to the commit message file as `$1`:

```yaml
hooks:
  commit-msg:
    commands:
      - name: validate
        run: node scripts/validate-msg.js $1
```

## Verbose logging

Two CLI options control the structured log output written to stderr:

| Option | Default | Description |
|--------|---------|-------------|
| `-v`, `--verbose` | off | Enable verbose logging. When omitted only INFO and ERROR entries are emitted. |
| `--verbose-template` | `[$t] [$k] ($l) $m` | Custom format for each log line (see below). |

### Log kinds

| Kind | Emitted when |
|------|-------------|
| `INFO` | Always (even without `-v`). Marks key milestones (config loaded, hook completed). |
| `VERBOSE` | Only with `-v`. Per-operation detail (resolving commands, running each command). |
| `ERROR` | Always. Failures before an early exit. |
| `TRACE` | Only with `-v`. Fine-grained internals (cache hits, exec strings, wave counts). |

### Log layers

| Layer | Subsystem |
|-------|-----------|
| `schema validation` | JSON-schema generation and validation (`githops schema sync`, `githops sync`). |
| `yaml resolve` | Config loading and include resolution. |
| `yaml exec` | Hook command execution (`githops check`). |

### Template tokens

| Token | Replaced with |
|-------|--------------|
| `$t`  | Timestamp — `HH:MM:SS.mmm` (UTC) |
| `$k`  | Kind — `INFO`, `VERBOSE`, `ERROR`, `TRACE` |
| `$l`  | Layer — `schema validation`, `yaml resolve`, `yaml exec` |
| `$m`  | Message text |

### Examples

Default output:
```
[12:34:56.789] [INFO] (yaml resolve) config loaded from githops.yaml
[12:34:56.791] [VERBOSE] (yaml resolve) resolved 3 command(s)
[12:34:56.792] [TRACE] (yaml exec) built 2 execution wave(s) for 3 command(s)
```

Minimal template — just kind and message:
```sh
githops check pre-commit -v --verbose-template "$k: $m"
```
```
INFO: config loaded from githops.yaml
VERBOSE: resolved 3 command(s)
```

JSON-friendly template:
```sh
githops sync -v --verbose-template '{"time":"$t","kind":"$k","layer":"$l","msg":"$m"}'
```
