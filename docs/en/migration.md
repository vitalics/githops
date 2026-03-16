# Migration Guide

This guide covers migrating to githops from Husky, lefthook, and pre-commit. The general approach is the same for all three: run `githops migrate` to generate an initial `githops.yaml` from your existing configuration, then review and refine the output.

## Automatic migration

```sh
githops migrate
```

githops detects your current hook manager by looking for configuration files in the repository root:

- `.husky/` directory → Husky
- `lefthook.yml` or `lefthook.yaml` → lefthook
- `.pre-commit-config.yaml` → pre-commit

It reads the existing configuration, converts it to `githops.yaml`, and writes the file. Your existing hook manager files are not deleted.

Review the generated file carefully. Automatic migration handles the common cases but may not capture everything — conditional logic in Husky shell scripts, for example, needs to be reviewed manually.

## Migrating from Husky

### Husky v9 structure

A typical Husky project has hook files in `.husky/`:

```sh
.husky/
  pre-commit    # shell script
  pre-push      # shell script
```

Each file is a plain shell script:

```sh
# .husky/pre-commit
npm run lint
npm run typecheck
```

### Equivalent githops.yaml

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
```

### Steps

1. Run `githops migrate` to generate an initial `githops.yaml`.
2. Review the generated file.
3. Run `githops sync` to install the githops-managed hooks.
4. Remove the `prepare` script from `package.json` that installed Husky, or replace it with `githops sync`.
5. Delete `.husky/` and remove `husky` from `devDependencies`.

```sh
# Remove husky
npm uninstall husky
rm -rf .husky
```

## Migrating from lefthook

### lefthook.yml structure

```yaml
pre-commit:
  parallel: true
  commands:
    lint:
      run: npm run lint
    typecheck:
      run: npm run typecheck

pre-push:
  commands:
    test:
      run: npm test
```

### Equivalent githops.yaml

```yaml
hooks:
  pre-commit:
    enabled: true
    parallel: true
    commands:
      - name: lint
        run: npm run lint
      - name: typecheck
        run: npm run typecheck

  pre-push:
    enabled: true
    commands:
      - name: test
        run: npm test
```

### Steps

1. Run `githops migrate`.
2. Review `githops.yaml`.
3. Run `githops sync`.
4. Remove lefthook from your project.

```sh
# On macOS with Homebrew
brew uninstall lefthook
# Or if installed via npm
npm uninstall @evilmartians/lefthook
```

## Migrating from pre-commit

pre-commit uses a plugin model where hooks reference external repositories. githops does not have a plugin system — you write the commands directly. Migration therefore means replacing plugin references with the equivalent commands installed in your project.

### .pre-commit-config.yaml example

```yaml
repos:
  - repo: https://github.com/pre-commit/mirrors-eslint
    rev: v8.56.0
    hooks:
      - id: eslint
  - repo: https://github.com/pre-commit/mirrors-prettier
    rev: v3.1.0
    hooks:
      - id: prettier
```

### Equivalent githops.yaml

```yaml
hooks:
  pre-commit:
    enabled: true
    commands:
      - name: eslint
        run: npx eslint .
      - name: prettier
        run: npx prettier --check .
```

### Steps

1. Identify which commands each pre-commit plugin runs. Most mirrors just call the tool's CLI.
2. Ensure those tools are installed in your project (`npm install --save-dev eslint prettier`).
3. Write the equivalent `githops.yaml` manually or run `githops migrate` for a starting point.
4. Run `githops sync`.
5. Remove pre-commit.

```sh
pip uninstall pre-commit
rm .pre-commit-config.yaml
```

## Manual migration checklist

After migrating from any tool:

- [ ] `githops.yaml` exists and `githops sync` runs without errors.
- [ ] `.git/hooks/` contains the hooks managed by githops (check with `ls .git/hooks/`).
- [ ] Old hook manager files are removed from the repository.
- [ ] Old hook manager is removed from `devDependencies` / system packages.
- [ ] `prepare` script in `package.json` (if any) calls `githops sync` instead of the old tool.
- [ ] `.gitignore` includes `.githops/cache` if you plan to use caching.
- [ ] Team members have been informed to run `githops sync` (or `npm install` if using the `prepare` script).
