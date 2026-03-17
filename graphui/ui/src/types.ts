export type CommandCacheConfig = {
  inputs: string[];
  key: string[];
};

export type IncludeSource = {
  source: "local" | "remote" | "git";
  ref: string;
  type: "json" | "toml" | "yaml";
  // local
  path?: string;
  // remote
  url?: string;
  // git
  rev?: string;
  file?: string;
};

export type CommandEntry = {
  isRef: boolean;
  refName: string;
  /** Display name. For refs: resolved from definition (may be overridden by nameOverride). */
  name: string;
  /** For inline: the shell command. For refs: the definition's default run (read-only). */
  run: string;
  /** Extra arguments appended to the definition's run command (refs only). */
  refArgs?: string;
  /** Explicit name label override for this ref use-site (refs only, empty = use def name). */
  nameOverride?: string;
  depends: string[];
  env: Record<string, string>;
  test: boolean;
  cache?: CommandCacheConfig | null;
  isInclude?: boolean;
  includeRef?: string;
  /** Dot-notation path into the included file (e.g. `scripts.lint`). */
  includePath?: string;
  /** Extra CLI arguments appended to the resolved command. */
  args?: string;
};

export type HookState = {
  name: string;
  description: string;
  category: string;
  configured: boolean;
  installed: boolean;
  enabled: boolean;
  parallel: boolean;
  commands: CommandEntry[];
};

export type UniqueCommand = {
  name: string;
  run: string;
  test: boolean;
  usedIn: string[];
};

export type DefCommand = {
  name: string;
  run: string;
  depends: string[];
  env: Record<string, string>;
  test: boolean;
};

export type DefinitionEntry = {
  name: string;
  type: "single" | "list";
  commands: DefCommand[];
};

export type CacheEntry = {
  key: string;
  ageMs: number;
};

export type CacheStatus = {
  enabled: boolean;
  dir: string;
  entries: CacheEntry[];
};

export type AppState = {
  hooks: HookState[];
  commands: UniqueCommand[];
  definitions: DefinitionEntry[];
  includes: IncludeSource[];
  configExists: boolean;
  cacheStatus: CacheStatus;
};

// CDP-style WebSocket request/response
export type WsResult = Record<string, unknown>;
