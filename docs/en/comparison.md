# githops vs Other Tools

There are several tools in this space. The table below compares them on dimensions that tend to matter when choosing one for a real project.

## Comparison table

| Feature                    | githops                     | Husky                | lefthook  | pre-commit    |
| -------------------------- | --------------------------- | -------------------- | --------- | ------------- |
| Configuration format       | YAML                        | JSON / shell scripts | YAML      | YAML          |
| Runtime dependency         | None (single binary)        | Node.js              | Go binary | Python        |
| Cross-platform             | Yes                         | Yes                  | Yes       | Yes           |
| Parallel execution         | Per-hook flag               | No                   | Yes       | No            |
| Dependency ordering        | Yes (`depends`)             | No                   | No        | No            |
| Smart caching              | Yes (file-hash based)       | No                   | No        | Yes (partial) |
| Reusable definitions       | Yes (`$ref`)                | No                   | No        | No            |
| YAML anchors               | Yes                         | No                   | No        | No            |
| Visual graph UI            | Yes                         | No                   | No        | No            |
| Self-update                | Yes (`githops self-update`) | No                   | No        | No            |
| Shell completions          | Yes                         | No                   | No        | No            |
| JSON Schema for config     | Yes                         | No                   | No        | No            |
| Migration from other tools | Yes (`githops migrate`)     | —                    | —         | —             |
| Windows MSI installer      | Yes                         | No                   | No        | No            |

## Notes on each tool

### Husky

Husky is the most widely used Git hook manager in the JavaScript ecosystem. It works by writing small shell scripts into `.git/hooks/` that delegate to scripts in your repository. Its main strength is deep integration with `npm`/`pnpm` workflows — hooks are installed automatically on `npm install` via the `prepare` lifecycle script.

The main limitation is that Husky only handles the plumbing. Each hook file is a plain shell script you write yourself, so you get no parallel execution, no dependency ordering, no caching, and no reuse between hooks. It also requires Node.js, which is a problem for non-JavaScript projects.

### lefthook

lefthook is a Go-based hook runner with a YAML configuration file. It supports parallel execution within a hook and is notably faster than Husky for large repositories. It has no runtime dependency once installed.

Compared to githops, lefthook does not have dependency ordering between commands within a hook, no content-based caching, no reusable command definitions, and no visual tooling. It is a good choice for teams that want something lightweight without the full feature set.

### pre-commit

pre-commit focuses on running checks from external repositories — you declare hook "plugins" hosted on GitHub and pre-commit fetches and runs them in isolated virtual environments. This is powerful for language-agnostic linters where you don't want to install the tool globally.

The trade-off is that pre-commit requires Python and manages its own environment per plugin. Startup overhead is higher, and the model of pulling plugins from remote repositories introduces a network dependency. It also only manages the `pre-commit` hook by default — support for other hooks exists but is secondary.

### githops

githops takes a different approach: it is not a plugin system or a framework. It is a structured YAML runner for the hooks you already have. You write the commands yourself, organize them with definitions and references, and githops handles execution ordering, parallelism, and caching. The visual graph makes the full picture visible at a glance.

If your project already has the commands you want to run — linting, formatting, type-checking, testing — githops gives you the structure to manage them across all your Git hooks without adding a runtime dependency.
