# External Includes

githops can import command scripts from external configuration files — local files in your repository, remote files fetched over HTTP, or files from remote Git repositories. This lets you centralise shared scripts in one place and reuse them across projects.

## Declaring an include

All includes go in the top-level `include:` list. Each entry has a `source` field (`local`, `remote`, or `git`) and a `ref` that names the include; you use this name in hook commands with `$include:`.

### Local file

Import scripts from a file in your repository (e.g. `package.json`, `Cargo.toml`, a shared scripts YAML):

```yaml
include:
  - source: local
    path: package.json
    type: json
    ref: packagejson
```

Supported types: `json`, `toml`, `yaml`.

### Remote file

Fetch a file over HTTP or HTTPS:

```yaml
include:
  - source: remote
    url: "https://example.com/shared-scripts.yaml"
    type: yaml
    ref: sharedscripts
```

The file is fetched each time the hook runs. The `type` field defaults to `yaml` if omitted.

### Git repository

Check out a single file from a remote Git repository at a specific revision:

```yaml
include:
  - source: git
    url: "https://github.com/org/repo.git"
    rev: main
    file: "ci/scripts.yaml"
    type: yaml
    ref: repotemplate
```

githops runs `git clone --depth=1` to a temporary directory and reads the specified file. The `type` field defaults to `yaml` if omitted.

## Using an include in a hook

Reference an include in any hook command with `$include:`:

```yaml
hooks:
  pre-commit:
    enabled: true
    commands:
      - $include: packagejson
        run: scripts.lint
```

The `run` field is a **dot-notation path** into the included file. githops navigates the path and uses the resulting string as the shell command to run.

For example, if `package.json` contains:

```json
{
  "scripts": {
    "lint": "eslint . --ext .ts"
  }
}
```

Then `run: scripts.lint` resolves to `eslint . --ext .ts`.

### Extra arguments

Use `args` to append extra CLI flags to the resolved command:

```yaml
- $include: packagejson
  run: scripts.lint
  args: "--fix"
```

This produces `eslint . --ext .ts --fix`.

### Environment variables

Use `env` to set environment variables for the invocation:

```yaml
- $include: packagejson
  run: scripts.lint
  args: "--fix"
  env:
    NODE_ENV: production
```

### Optional: display name

Add a `name` field to override the display label in output and the graph UI:

```yaml
- $include: packagejson
  run: scripts.lint
  name: ESLint
```

Without `name`, githops uses the last segment of the dot-path (`lint` in this case).

## Supported file formats

| Type   | File examples                    | Navigation                           |
| ------ | -------------------------------- | ------------------------------------ |
| `json` | `package.json`                   | `scripts.lint`, `dependencies.react` |
| `toml` | `Cargo.toml`                     | `package.version`, `scripts.build`   |
| `yaml` | `scripts.yaml`, `.gitlab-ci.yml` | `jobs.lint.script`, `scripts.build`  |

The value at the resolved path must be a string.

## Complete example

```yaml
include:
  - source: local
    path: package.json
    type: json
    ref: pkg

  - source: remote
    url: "https://example.com/shared.yaml"
    ref: shared

  - source: git
    url: "https://github.com/org/hooks.git"
    rev: v2.1.0
    file: "hooks/common.yaml"
    ref: common

hooks:
  pre-commit:
    enabled: true
    parallel: true
    commands:
      - $include: pkg
        run: scripts.lint
        args: "--fix"
        name: lint
      - $include: pkg
        run: scripts.typecheck
        name: typecheck
      - $include: shared
        run: scripts.format-check
        env:
          NODE_ENV: production

  pre-push:
    enabled: true
    commands:
      - $include: common
        run: scripts.test
```

## Notes

- Remote and git includes require network access at hook run time.
- Git includes require `git` to be installed and available on `PATH`.
- The temporary directory used for git clones is `{system-temp}/githops-git-{hash}` and is reused within a single githops run.
- Include entries are visible and editable in the **Includes** tab of `githops graph`.
