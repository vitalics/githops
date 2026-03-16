# Introduction

## What is githops?

githops is a command-line tool for managing Git hooks through a single YAML configuration file. Instead of scattering shell scripts across `.git/hooks/` and hoping that every developer on the team has them set up correctly, you declare your hooks in `githops.yaml`, commit that file, and run `githops sync` once. Everyone on the team gets the same hooks automatically.

## The problem

Git hooks are powerful. They can run linters before a commit, validate commit messages, run tests before a push, and enforce project conventions without any CI round-trip. The trouble is that Git hooks are not committed to the repository — `.git/hooks/` is intentionally excluded from version control.

In practice this leads to:

- **Inconsistency.** New team members miss hooks entirely because nobody told them to set up the scripts manually.
- **Duplication.** The same `npm run lint` command ends up copy-pasted into `pre-commit`, `pre-push`, and occasionally `commit-msg`.
- **Fragility.** Shell scripts in `.git/hooks/` have no structure, no dependency ordering, and fail silently on Windows paths.
- **Opacity.** There is no single place to see which hooks are active, what they run, or why.

Tools like Husky, lefthook, and pre-commit all solve parts of this problem, but they each make trade-offs that may not fit your project. githops is designed around a different set of priorities.

## Core ideas

**One file, all hooks.** Every hook in every Git event is configured in `githops.yaml`. You read one file to understand everything that runs in your repository.

**Definitions for reuse.** Common commands — running a linter, building the project, checking the lockfile — can be extracted as named definitions and referenced from multiple hooks with `$ref`. No copy-pasting.

**YAML anchors and templates.** Standard YAML anchors (`&anchor` / `*alias`) work out of the box for sharing configuration fragments between hooks.

**Parallel execution.** Per-hook `parallel: true` runs commands concurrently and waits for all of them, which is important for slow linters or type-checkers.

**Smart caching.** Commands can declare the files they depend on. githops skips the command if none of those files changed since the last successful run.

**Visual graph.** `githops graph` opens a browser-based dependency graph so you can see exactly how hooks, commands, and definitions relate to each other, and edit them interactively.

**Cross-platform.** githops is a single Rust binary with no runtime dependencies. It works on macOS, Linux, and Windows without Node.js, Python, or any other interpreter.

## Non-goals

githops does not replace your CI pipeline. The hooks it manages run locally on developer machines. For server-side enforcement, use your CI system — githops can help you keep local and CI checks in sync by sharing the same command definitions, but it does not run in CI itself.
