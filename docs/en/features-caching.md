# Caching

githops supports content-based caching for individual commands. When a command has a cache configuration, githops computes a hash of the files the command depends on and skips the command if the hash has not changed since the last successful run.

This is particularly useful for slow checks — type-checking, test runs, build steps — where you want to avoid re-running them when unrelated files change.

## Enabling global caching

Caching must be enabled globally before per-command cache configurations take effect:

```yaml
cache:
  enabled: true
```

Optionally, change the cache directory (default: `.githops/cache`):

```yaml
cache:
  enabled: true
  dir: .githops/cache
```

Add `.githops/cache` to your `.gitignore`:

```
.githops/cache
```

## Per-command cache configuration

Add a `cache` block to any inline command:

```yaml
hooks:
  pre-push:
    commands:
      - name: typecheck
        run: npx tsc --noEmit
        cache:
          inputs:
            - "src/**/*.ts"
            - "src/**/*.tsx"
            - tsconfig.json
          key:
            - package.json
```

### `inputs`

A list of glob patterns. githops hashes the content of all matching files. If the hash matches the stored hash from the last successful run, the command is skipped.

```yaml
cache:
  inputs:
    - "src/**/*.ts"
    - "src/**/*.tsx"
```

### `key`

An optional list of additional files whose content is included in the cache key. Use this for configuration files that affect the command's output but are not covered by `inputs`:

```yaml
cache:
  inputs:
    - "src/**/*.ts"
  key:
    - tsconfig.json
    - package-lock.json
```

The full cache key is the SHA-256 hash of the combined content of all `inputs` and `key` files.

## How it works

1. When the command is about to run, githops globs all `inputs` and `key` patterns.
2. The content of matched files is hashed with SHA-256.
3. githops checks for a marker file in the cache directory named `<command-name>.<hash>.ok`.
4. If the marker file exists, the command is skipped and githops reports "cached".
5. If the marker file does not exist, the command runs normally.
6. If the command exits with code 0, githops writes the marker file.
7. If the command fails, no marker is written and the command will run again next time.

## Cache invalidation

The cache is automatically invalidated when any `inputs` or `key` file changes. You do not need to manually clear the cache for this.

To clear all cache entries manually:

```sh
githops cache clear
```

Or from the graph UI: open the Cache tab and click **Clear all**.

## Viewing cached entries

```sh
githops cache list
```

This shows each cached command, its key, and how long ago it was cached.

## Combining caching with parallelization

Caching and parallel execution work together. In a parallel hook, githops checks the cache for each command independently. Commands that are cached are skipped immediately, so already-passing checks never block the parallel execution of commands that do need to run.

## Example: efficient pre-push hook

```yaml
cache:
  enabled: true

hooks:
  pre-push:
    enabled: true
    parallel: true
    commands:
      - name: typecheck
        run: npx tsc --noEmit
        cache:
          inputs:
            - "src/**/*.ts"
            - "src/**/*.tsx"
          key:
            - tsconfig.json

      - name: test
        run: npx jest --passWithNoTests
        cache:
          inputs:
            - "src/**/*.test.ts"
            - "src/**/*.ts"
          key:
            - jest.config.ts
            - package.json

      - name: lint
        run: npx eslint src
        cache:
          inputs:
            - "src/**/*.ts"
            - "src/**/*.tsx"
          key:
            - .eslintrc.json
            - package.json
```

With this configuration, pushing a commit that only changes a README file skips all three checks because none of their `inputs` changed. Pushing a change to a TypeScript file runs typecheck and test (which depend on `.ts` files) but may skip lint if the `.eslintrc.json` and the changed `.ts` files match the cached hash.
