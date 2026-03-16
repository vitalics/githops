# Parallelization

By default, githops runs commands in a hook sequentially in the order they are declared (taking `depends` constraints into account). For hooks that run multiple slow checks — type-checking, linting, tests — sequential execution wastes time. The `parallel` flag changes that.

## Enabling parallel execution

Set `parallel: true` on any hook:

```yaml
hooks:
  pre-push:
    enabled: true
    parallel: true
    commands:
      - name: lint
        run: npm run lint
      - name: typecheck
        run: npx tsc --noEmit
      - name: test
        run: npm test
```

With `parallel: true`, githops launches all commands concurrently and waits for all of them to finish before allowing the hook to exit. If any command fails, the hook fails and the Git operation is aborted.

## Interaction with `depends`

`parallel: true` does not ignore `depends`. Commands that declare dependencies still wait for those dependencies before starting. This means you can have a mix of concurrent and sequential stages within a single hook:

```yaml
hooks:
  pre-push:
    enabled: true
    parallel: true
    commands:
      - name: install
        run: npm ci

      - name: lint
        run: npm run lint
        depends:
          - install

      - name: typecheck
        run: npx tsc --noEmit
        depends:
          - install

      - name: test
        run: npm test
        depends:
          - install
```

In this configuration:

1. `install` runs first.
2. Once `install` succeeds, `lint`, `typecheck`, and `test` all start concurrently.
3. The hook succeeds only when all three finish successfully.

This gives you a two-stage pipeline: a single setup phase followed by a fully parallel check phase.

## Execution model

githops builds a dependency graph for all commands in the hook. Commands with no unresolved dependencies are eligible to run. When `parallel: true`, all eligible commands start at the same time. As commands finish, newly eligible commands start immediately. This continues until all commands have run or any command fails.

When `parallel: false` (the default), eligible commands run one at a time in declaration order.

## Output

When running in parallel, githops prefixes each line of output with the command name so you can distinguish which command produced which output:

```
[lint]      Running ESLint...
[typecheck] Running tsc...
[test]      Running Jest...
[lint]      Finished in 2.1s
[typecheck] error TS2345: Argument of type...
[test]      Finished in 4.8s
```

If a command fails, githops prints its full output (including stderr) and reports the exit code.

## When to use parallelization

Parallel execution is most valuable when:

- Your hook runs multiple independent checks (linting, type-checking, testing).
- Each check takes more than a second or two.
- The checks do not write to the same files.

Avoid `parallel: true` when commands must write shared state in a specific order, or when resource contention (CPU, memory, disk I/O) would make concurrent execution slower than sequential.

## Checking execution time

Run githops with `--verbose` to see how long each command took:

```sh
githops check --verbose
```

This helps identify which commands are the bottleneck and whether parallelization is worth enabling for a given hook.
