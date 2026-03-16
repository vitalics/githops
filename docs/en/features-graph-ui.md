# Graph UI

`githops graph` opens a visual, interactive interface for exploring and editing your hook configuration. It runs a local web server and opens your browser automatically when you pass `--open`.

```sh
githops graph --open
```

## What the graph shows

The **Flow** tab displays a directed graph where:

- **Hook nodes** (labeled boxes at the top level) represent each configured Git hook.
- **Command nodes** appear below each hook and show all commands in that hook.
- **Definition ref nodes** are visually distinct and show which commands are references to shared definitions rather than inline commands.
- **Edges** connect commands to their dependencies, showing the execution order.

You can see at a glance which hooks are configured, which commands they share via definitions, and what the execution order is within each hook.

## Tabs

### Hooks

The Hooks tab shows all supported Git hooks. Configured hooks are highlighted. Click any hook to open its editor pane, where you can:

- Enable or disable the hook.
- Toggle parallel execution.
- Add, reorder, and remove commands.
- Add inline commands or definition references.
- Edit the `run` command, `depends`, `env`, and cache settings for each command.

Changes are saved to `githops.yaml` immediately on save — there is no separate "write" step.

### Commands

The Commands tab lists all unique inline commands across all configured hooks. This is useful for finding duplicate commands that could be extracted into definitions. You can edit a command's `run` value here and the change propagates to every hook that uses it.

### Definitions

The Definitions tab shows all named definitions. You can create new definitions, edit existing ones, change the definition type (single command vs. command list), and delete definitions. If a definition is deleted, all `$ref` entries pointing to it are removed from all hooks automatically.

### Flow

The Flow tab shows the dependency graph described above. You can:

- **Pan** by dragging the canvas.
- **Zoom** with the scroll wheel or the zoom controls in the corner.
- **Drag a handle** between two command nodes to add a dependency edge.
- **Select an edge and press Delete** to remove a dependency.

Changes made in the flow view are written to `githops.yaml` immediately.

### Cache

The Cache tab shows the current cache configuration and all cached entries. You can:

- Enable or disable global caching.
- Change the cache directory.
- Clear all cache entries with one click.
- See each cached command, its cache key (SHA-256 hash), and how long ago it was cached.

## Live reload

The UI polls `githops.yaml` for changes every second. If you edit the file directly in your editor while the graph is open, the UI updates automatically without a page refresh.

## Starting without auto-open

```sh
githops graph
```

githops prints the URL to the terminal (`http://127.0.0.1:7890` by default). Open it in any browser manually. If port 7890 is taken, githops picks a random available port.

## Stopping the server

Press `Ctrl+C` in the terminal where `githops graph` is running.
