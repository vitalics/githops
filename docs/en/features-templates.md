# Templates: Definitions, Refs, and YAML Anchors

One of the most common pain points with Git hook tools is duplication. The same linting command ends up in `pre-commit`, the same build command in `pre-push`, and when you need to change the flag you pass to the linter, you hunt through every hook to update all the copies.

githops solves this at two levels: **definitions** for named, reusable command groups, and **YAML anchors** for sharing raw configuration fragments.

## Definitions

A definition is a named command or list of commands declared under the top-level `definitions` key. Any hook can reference a definition by name instead of duplicating the command inline.

### Declaring a definition

```yaml
definitions:
  lint:
    name: ESLint
    run: npx eslint . --ext .ts,.tsx

  typecheck:
    name: TypeScript check
    run: npx tsc --noEmit
```

### Using a definition in a hook

```yaml
hooks:
  pre-commit:
    enabled: true
    commands:
      - $ref: lint
      - $ref: typecheck
        depends:
          - lint
```

When githops runs the hook, it expands each `$ref` into the command declared in `definitions`. Changing the `run` value in the definition updates all hooks that reference it.

### Passing extra arguments

You can append extra arguments to the definition's `run` command at the call site:

```yaml
hooks:
  pre-commit:
    commands:
      - $ref: lint
        args: "--fix"
```

This runs `npx eslint . --ext .ts,.tsx --fix` for this hook, while other hooks that reference `lint` without `args` run the original command.

### Overriding the display name

The `name` field on a `$ref` entry overrides the display name shown in githops output and the graph UI:

```yaml
- $ref: lint
  name: "lint (with autofix)"
  args: "--fix"
```

### Multi-command definitions

A definition can be a list of commands with their own dependencies:

```yaml
definitions:
  full-check:
    - name: install
      run: npm ci
    - name: lint
      run: npx eslint .
      depends:
        - install
    - name: test
      run: npm test
      depends:
        - lint
```

Referencing `$ref: full-check` in a hook expands to all three commands, preserving the dependency graph.

## YAML anchors

Standard YAML anchors work anywhere in `githops.yaml`. They are resolved by the YAML parser before githops reads the file, so they can be used for any structural fragment — environment variables, sets of commands, nested options.

### Sharing environment variables

```yaml
x-common-env: &common-env
  NODE_ENV: production
  CI: "true"

hooks:
  pre-push:
    commands:
      - name: build
        run: npm run build
        env: *common-env

  pre-commit:
    commands:
      - name: lint
        run: npm run lint
        env: *common-env
```

### Sharing a command block

```yaml
x-lint: &lint-cmd
  name: lint
  run: npx eslint .

hooks:
  pre-commit:
    commands:
      - <<: *lint-cmd

  pre-merge-commit:
    commands:
      - <<: *lint-cmd
```

The `<<` merge key is standard YAML and is supported by all YAML 1.1 parsers, including the one githops uses.

## Definitions vs YAML anchors

Both solve the duplication problem, but they have different trade-offs:

| | Definitions | YAML anchors |
|---|---|---|
| Visible in graph UI | Yes | No |
| Can be referenced by name | Yes | No (structural only) |
| Supports `$ref` with `args` | Yes | No |
| Works across files | No (single file) | No (single file) |
| YAML-standard | No (githops extension) | Yes |
| Editor autocomplete | Yes (via schema) | Depends on editor |

Use definitions when you want named, reusable commands that are visible in the graph and can be referenced with arguments. Use YAML anchors for raw structural sharing — repeating environment maps, common option sets, or configuration fragments that are not themselves commands.
