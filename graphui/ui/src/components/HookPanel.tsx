import { useState, useEffect, useRef } from "react";
import type { CommandCacheConfig, CommandEntry, DefinitionEntry, HookState, IncludeSource } from "../types";
import type { WsResult } from "../types";
import { t } from "../i18n";
import { Button } from "./ui/button";
import { Input } from "./ui/input";
import { Switch } from "./ui/switch";
import { Checkbox } from "./ui/checkbox";
import { Field, Label } from "./ui/fieldset";
import { Badge } from "./ui/badge";

type SendFn = (method: string, params?: Record<string, unknown>) => Promise<WsResult>;
type DraftCmd = CommandEntry & { _key: string };

// ── HookPanel ─────────────────────────────────────────────────────────────────

type Props = {
  hooks: HookState[];
  definitions: DefinitionEntry[];
  includes: IncludeSource[];
  send: SendFn;
  onNavigateToDefs: (defName: string) => void;
};

export function HookPanel({ hooks, definitions, includes, send, onNavigateToDefs }: Props) {
  const [selected, setSelected] = useState<string | null>(null);
  const isDirtyRef = useRef(false);

  function handleSelect(name: string) {
    if (selected === name) return;
    if (isDirtyRef.current && !window.confirm(t("confirm_discard"))) return;
    isDirtyRef.current = false;
    setSelected(name);
  }

  const hook = hooks.find((h) => h.name === selected) ?? null;

  const byCategory = hooks.reduce<Record<string, HookState[]>>((acc, h) => {
    (acc[h.category] ??= []).push(h);
    return acc;
  }, {});

  return (
    <div className="flex h-full">
      {/* Left: hook list */}
      <div className="w-52 shrink-0 border-r border-[var(--color-border)] overflow-y-auto p-3 space-y-4">
        {Object.entries(byCategory).map(([cat, items]) => (
          <section key={cat}>
            <div className="text-[9px] font-bold uppercase tracking-widest text-[var(--color-muted)] mb-1.5">
              {cat}
            </div>
            <div className="flex flex-col gap-0.5">
              {items.map((h) => (
                <HookListItem
                  key={h.name}
                  hook={h}
                  selected={selected === h.name}
                  onClick={() => handleSelect(h.name)}
                />
              ))}
            </div>
          </section>
        ))}
      </div>

      {/* Right: editor */}
      <div className="flex-1 overflow-y-auto">
        {!hook ? (
          <div className="flex items-center justify-center h-full text-[var(--color-muted)] text-sm whitespace-pre-line text-center">
            {t("select_hook")}
          </div>
        ) : (
          <HookEditor
            key={`${hook.name}-${hook.configured}`}
            hook={hook}
            definitions={definitions}
            includes={includes}
            send={send}
            onDirtyChange={(d) => { isDirtyRef.current = d; }}
            onNavigateToDefs={onNavigateToDefs}
          />
        )}
      </div>
    </div>
  );
}

// ── HookListItem ──────────────────────────────────────────────────────────────

function HookListItem({
  hook: h,
  selected,
  onClick,
}: {
  hook: HookState;
  selected: boolean;
  onClick: () => void;
}) {
  const active = h.configured && h.enabled;
  const needsSync = h.configured && h.enabled && !h.installed;

  return (
    <button
      onClick={onClick}
      className={`flex items-center gap-2 w-full text-left px-2 py-1.5 rounded text-xs transition-colors
        ${selected
          ? "bg-[var(--color-accent-dim)] text-[var(--color-accent)]"
          : active
          ? "text-[var(--color-text)] hover:bg-[var(--color-border)]"
          : "text-[var(--color-muted)] hover:bg-[var(--color-border)]"
        }`}
    >
      <span
        className={`w-1.5 h-1.5 rounded-full shrink-0 ${
          active
            ? "bg-[var(--color-accent)]"
            : h.configured
            ? "bg-[var(--color-muted)]"
            : "bg-[var(--color-border)]"
        }`}
      />
      <span className="truncate">{h.name}</span>
      {h.installed && (
        <Badge color="accent" className="ml-auto shrink-0">{t("installed")}</Badge>
      )}
      {needsSync && !h.installed && (
        <Badge color="yellow" className="ml-auto shrink-0">{t("needs_sync")}</Badge>
      )}
    </button>
  );
}

// ── HookEditor ────────────────────────────────────────────────────────────────

function HookEditor({
  hook,
  definitions,
  includes,
  send,
  onDirtyChange,
  onNavigateToDefs,
}: {
  hook: HookState;
  definitions: DefinitionEntry[];
  includes: IncludeSource[];
  send: SendFn;
  onDirtyChange: (dirty: boolean) => void;
  onNavigateToDefs: (defName: string) => void;
}) {
  const keyRef = useRef(0);
  const [enabled, setEnabled] = useState(hook.enabled);
  const [parallel, setParallel] = useState(hook.parallel);
  const [cmds, setCmds] = useState<DraftCmd[]>(() =>
    hook.commands.map((c) => ({ ...c, _key: String(keyRef.current++) })),
  );
  const [saving, setSaving] = useState(false);

  const origJson = JSON.stringify(hook.commands);
  const dirty =
    enabled !== hook.enabled ||
    parallel !== hook.parallel ||
    JSON.stringify(cmds.map(stripKey)) !== origJson;

  const onDirtyRef = useRef(onDirtyChange);
  onDirtyRef.current = onDirtyChange;
  useEffect(() => { onDirtyRef.current(dirty); }, [dirty]);

  function updateCmd(key: string, patch: Partial<DraftCmd>) {
    setCmds((prev) => prev.map((c) => (c._key === key ? { ...c, ...patch } : c)));
  }
  function deleteCmd(key: string) {
    setCmds((prev) => prev.filter((c) => c._key !== key));
  }
  function moveCmd(key: string, dir: -1 | 1) {
    setCmds((prev) => {
      const i = prev.findIndex((c) => c._key === key);
      if (i < 0) return prev;
      const j = i + dir;
      if (j < 0 || j >= prev.length) return prev;
      const next = [...prev];
      [next[i], next[j]] = [next[j], next[i]];
      return next;
    });
  }
  function addInline() {
    const key = String(keyRef.current++);
    setCmds((prev) => [
      ...prev,
      { _key: key, isRef: false, refName: "", name: "", run: "", depends: [], env: {}, test: false },
    ]);
  }
  function addRef(refName: string) {
    const key = String(keyRef.current++);
    setCmds((prev) => [
      ...prev,
      { _key: key, isRef: true, refName, name: refName, run: "", depends: [], env: {}, test: false },
    ]);
  }
  function addInclude(refName: string) {
    const key = String(keyRef.current++);
    setCmds((prev) => [
      ...prev,
      { _key: key, isRef: false, refName: "", isInclude: true, includeRef: refName, includePath: "", args: "", name: "", run: "", depends: [], env: {}, test: false },
    ]);
  }

  async function handleSave() {
    setSaving(true);
    try {
      await send("hook.update", {
        hook: hook.name,
        enabled,
        parallel,
        commands: cmds.map(stripKey),
      });
    } finally {
      setSaving(false);
    }
  }

  function handleCancel() {
    setEnabled(hook.enabled);
    setParallel(hook.parallel);
    setCmds(hook.commands.map((c) => ({ ...c, _key: String(keyRef.current++) })));
  }

  async function handleRemove() {
    if (!window.confirm(t("confirm_discard"))) return;
    await send("hook.remove", { hook: hook.name });
  }

  const inlineNames = cmds.filter((c) => !c.isRef && c.name).map((c) => c.name);

  return (
    <div className="p-5 max-w-2xl">
      {/* Hook header */}
      <div className="mb-4">
        <h2 className="text-sm font-bold text-[var(--color-accent)]">{hook.name}</h2>
        <p className="text-[11px] text-[var(--color-muted)] mt-0.5">
          {t(`hook_desc.${hook.name}`)}
        </p>
      </div>

      {/* Toggles */}
      <div className="flex gap-6 mb-4">
        <label className="flex items-center gap-2 text-xs text-[var(--color-muted)] cursor-pointer select-none">
          <Switch checked={enabled} onChange={setEnabled} />
          {t("enabled")}
        </label>
        <label className="flex items-center gap-2 text-xs text-[var(--color-muted)] cursor-pointer select-none">
          <Switch checked={parallel} onChange={setParallel} />
          {t("parallel")}
        </label>
      </div>

      {/* Commands */}
      <div className="space-y-2 mb-4">
        {cmds.map((cmd, i) => (
          <CmdCard
            key={cmd._key}
            cmd={cmd}
            index={i}
            total={cmds.length}
            inlineNames={inlineNames}
            definitions={definitions}
            includes={includes}
            onChange={(patch) => updateCmd(cmd._key, patch)}
            onDelete={() => deleteCmd(cmd._key)}
            onMove={(dir) => moveCmd(cmd._key, dir)}
            onNavigateToDefs={onNavigateToDefs}
          />
        ))}
        <AddCmdRow definitions={definitions} includes={includes} onAddInline={addInline} onAddRef={addRef} onAddInclude={addInclude} />
      </div>

      {/* Footer */}
      <div className="flex gap-2 pt-3 border-t border-[var(--color-border)]">
        <Button onClick={handleSave} disabled={saving || !dirty}>{t("save")}</Button>
        <Button outline onClick={handleCancel} disabled={!dirty}>{t("cancel")}</Button>
        {hook.configured && (
          <Button color="red" outline onClick={handleRemove} className="ml-auto">
            {t("remove_hook")}
          </Button>
        )}
      </div>
    </div>
  );
}

// ── CmdCard ───────────────────────────────────────────────────────────────────

function CmdCard({
  cmd,
  index,
  total,
  inlineNames,
  definitions,
  includes,
  onChange,
  onDelete,
  onMove,
  onNavigateToDefs,
}: {
  cmd: DraftCmd;
  index: number;
  total: number;
  inlineNames: string[];
  definitions: DefinitionEntry[];
  includes: IncludeSource[];
  onChange: (patch: Partial<DraftCmd>) => void;
  onDelete: () => void;
  onMove: (dir: -1 | 1) => void;
  onNavigateToDefs: (defName: string) => void;
}) {
  const otherNames = inlineNames.filter((n) => n !== cmd.name);

  return (
    <div className="border border-[var(--color-border)] rounded-lg p-3 bg-[var(--color-surface)] space-y-2">
      {/* Card controls */}
      <div className="flex items-center gap-1 justify-end -mt-1 -mr-1">
        <button
          onClick={() => onMove(-1)}
          disabled={index === 0}
          className="text-[var(--color-muted)] hover:text-[var(--color-text)] disabled:opacity-25 text-xs w-5 h-5"
          title="Up"
        >▲</button>
        <button
          onClick={() => onMove(1)}
          disabled={index === total - 1}
          className="text-[var(--color-muted)] hover:text-[var(--color-text)] disabled:opacity-25 text-xs w-5 h-5"
          title="Down"
        >▼</button>
        <button
          onClick={onDelete}
          className="text-[var(--color-muted)] hover:text-red-400 text-xs w-5 h-5"
        >×</button>
      </div>

      {cmd.isInclude ? (
        /* Include entry */
        <div className="space-y-2">
          <div className="flex items-center gap-2">
            <span className="text-[10px] text-[var(--color-muted)] shrink-0">{t("include_ref_label")}:</span>
            <select
              value={cmd.includeRef ?? ""}
              onChange={(e) => onChange({ includeRef: e.target.value })}
              className="flex-1 bg-[var(--color-canvas)] border border-[var(--color-border)] rounded px-2 py-1 text-xs text-[var(--color-text)] focus:outline-none focus:border-[var(--color-accent)]"
            >
              {includes.map((inc) => (
                <option key={inc.ref} value={inc.ref}>{inc.ref}</option>
              ))}
            </select>
          </div>
          <Field>
            <Label>{t("include_run")}</Label>
            <Input
              value={cmd.includePath ?? ""}
              onChange={(e) => onChange({ includePath: e.target.value })}
              placeholder="scripts.lint"
              className="font-mono"
            />
          </Field>
          <Field>
            <Label>{t("include_args")}</Label>
            <Input
              value={cmd.args ?? ""}
              onChange={(e) => onChange({ args: e.target.value })}
              placeholder="--fix"
              className="font-mono"
            />
          </Field>
          <Field>
            <Label>{t("name")}</Label>
            <Input
              value={cmd.name ?? ""}
              onChange={(e) => onChange({ name: e.target.value })}
              placeholder={t("name")}
            />
          </Field>
        </div>
      ) : cmd.isRef ? (
        /* Ref entry */
        <div className="space-y-2">
          <div className="flex items-center gap-2">
            <span className="text-[10px] text-[var(--color-muted)] shrink-0">{t("ref_label")}:</span>
            <select
              value={cmd.refName}
              onChange={(e) => onChange({ refName: e.target.value, name: e.target.value, refArgs: "", nameOverride: "" })}
              className="flex-1 bg-[var(--color-canvas)] border border-[var(--color-border)] rounded px-2 py-1 text-xs text-[var(--color-text)] focus:outline-none focus:border-[var(--color-accent)]"
            >
              {definitions.map((d) => (
                <option key={d.name} value={d.name}>{d.name}</option>
              ))}
            </select>
            <button
              onClick={() => onNavigateToDefs(cmd.refName)}
              className="text-[10px] text-[var(--color-accent)] hover:underline shrink-0"
            >
              {t("edit_in_defs")}
            </button>
          </div>
          <RefOverrides cmd={cmd} definitions={definitions} onChange={onChange} />
        </div>
      ) : (
        /* Inline entry */
        <div className="space-y-2">
          <div className="flex gap-2">
            <Field className="w-36">
              <Label>{t("name")}</Label>
              <Input
                value={cmd.name}
                onChange={(e) => onChange({ name: e.target.value })}
                placeholder="name"
              />
            </Field>
            <Field className="flex-1">
              <Label>{t("run")}</Label>
              <Input
                value={cmd.run}
                onChange={(e) => onChange({ run: e.target.value })}
                className="font-mono"
                placeholder="shell command"
              />
            </Field>
          </div>
          {otherNames.length > 0 && (
            <Field>
              <Label>{t("depends_on")}</Label>
              <div className="flex flex-wrap gap-3">
                {otherNames.map((n) => (
                  <label key={n} className="flex items-center gap-1.5 text-xs text-[var(--color-muted)] cursor-pointer">
                    <Checkbox
                      checked={cmd.depends.includes(n)}
                      onChange={(checked) => {
                        const next = checked
                          ? [...cmd.depends, n]
                          : cmd.depends.filter((d) => d !== n);
                        onChange({ depends: next });
                      }}
                    />
                    {n}
                  </label>
                ))}
              </div>
            </Field>
          )}
          <label className="flex items-center gap-1.5 text-xs text-[var(--color-muted)] cursor-pointer">
            <Checkbox
              checked={cmd.test}
              onChange={(checked) => onChange({ test: checked })}
            />
            {t("test_only")}
          </label>
          <CacheSection cache={cmd.cache ?? null} onChange={(c) => onChange({ cache: c })} />
        </div>
      )}
    </div>
  );
}

// ── RefOverrides ──────────────────────────────────────────────────────────────

function RefOverrides({
  cmd,
  definitions,
  onChange,
}: {
  cmd: DraftCmd;
  definitions: DefinitionEntry[];
  onChange: (patch: Partial<DraftCmd>) => void;
}) {
  const hasOverrides = !!(cmd.refArgs || cmd.nameOverride);
  const [open, setOpen] = useState(hasOverrides);

  const def = definitions.find((d) => d.name === cmd.refName);
  const defRun = def?.type === "single" && def.commands.length > 0 ? def.commands[0].run : "";
  const defName = def?.type === "single" && def.commands.length > 0 ? def.commands[0].name : cmd.refName;

  const previewRun = defRun
    ? cmd.refArgs
      ? `${defRun} ${cmd.refArgs}`
      : defRun
    : "";

  return (
    <div className="border-t border-[var(--color-border)] pt-2 mt-1">
      <button
        type="button"
        onClick={() => setOpen((o) => !o)}
        className="flex items-center gap-1.5 text-xs text-[var(--color-muted)] hover:text-[var(--color-text)] cursor-pointer select-none transition-colors"
      >
        <span className={`text-[9px] transition-transform ${open ? "rotate-90" : ""}`}>▶</span>
        <span>{t("overrides")}</span>
        {hasOverrides && <Badge color="accent" className="ml-1">ON</Badge>}
      </button>
      {open && (
        <div className="mt-2 space-y-2 pl-3 border-l border-[var(--color-border)]">
          <Field>
            <Label>{t("ref_args")}</Label>
            <Input
              value={cmd.refArgs ?? ""}
              onChange={(e) => onChange({ refArgs: e.target.value })}
              placeholder={t("ref_args_placeholder")}
              className="font-mono"
            />
          </Field>
          {previewRun && (
            <div className="space-y-0.5">
              <span className="text-[9px] uppercase tracking-widest text-[var(--color-muted)]">
                {t("preview")}
              </span>
              <p className="font-mono text-[10px] text-[var(--color-accent)] bg-[var(--color-accent-dim)] rounded px-2 py-1 truncate">
                {previewRun}
              </p>
            </div>
          )}
          <Field>
            <Label>{t("name_override")}</Label>
            <Input
              value={cmd.nameOverride ?? ""}
              onChange={(e) => onChange({ nameOverride: e.target.value })}
              placeholder={defName}
            />
          </Field>
        </div>
      )}
    </div>
  );
}

// ── CacheSection ──────────────────────────────────────────────────────────────

function CacheSection({
  cache,
  onChange,
}: {
  cache: CommandCacheConfig | null;
  onChange: (c: CommandCacheConfig | null) => void;
}) {
  const [open, setOpen] = useState(!!cache);

  return (
    <div className="border-t border-[var(--color-border)] pt-2 mt-1">
      <button
        type="button"
        onClick={() => setOpen((o) => !o)}
        className="flex items-center gap-1.5 text-xs text-[var(--color-muted)] hover:text-[var(--color-text)] cursor-pointer select-none transition-colors"
      >
        <span className={`text-[9px] transition-transform ${open ? "rotate-90" : ""}`}>▶</span>
        <span>{t("cache")}</span>
        {cache && <Badge color="accent" className="ml-1">ON</Badge>}
      </button>
      {open && (
        <div className="mt-2 space-y-2 pl-3 border-l border-[var(--color-border)]">
          <label className="flex items-center gap-1.5 text-xs text-[var(--color-muted)] cursor-pointer">
            <Checkbox
              checked={!!cache}
              onChange={(checked) => onChange(checked ? { inputs: [], key: [] } : null)}
            />
            {t("cache_enable")}
          </label>
          {cache && (
            <>
              <Field>
                <Label>{t("cache_inputs")}</Label>
                <Input
                  value={cache.inputs.join(", ")}
                  onChange={(e) =>
                    onChange({
                      ...cache,
                      inputs: e.target.value.split(",").map((s) => s.trim()).filter(Boolean),
                    })
                  }
                  placeholder="src/**/*.rs, Cargo.toml"
                />
              </Field>
              <Field>
                <Label>{t("cache_keys")}</Label>
                <Input
                  value={cache.key.join(", ")}
                  onChange={(e) =>
                    onChange({
                      ...cache,
                      key: e.target.value.split(",").map((s) => s.trim()).filter(Boolean),
                    })
                  }
                  placeholder="v1, $MY_ENV_VAR"
                />
              </Field>
            </>
          )}
        </div>
      )}
    </div>
  );
}

// ── AddCmdRow ─────────────────────────────────────────────────────────────────

function AddCmdRow({
  definitions,
  includes,
  onAddInline,
  onAddRef,
  onAddInclude,
}: {
  definitions: DefinitionEntry[];
  includes: IncludeSource[];
  onAddInline: () => void;
  onAddRef: (refName: string) => void;
  onAddInclude: (refName: string) => void;
}) {
  const [refSel, setRefSel] = useState(definitions[0]?.name ?? "");
  const [incSel, setIncSel] = useState(includes[0]?.ref ?? "");

  return (
    <div className="flex items-center gap-2 pt-1 flex-wrap">
      <button
        onClick={onAddInline}
        className="px-2 py-1 text-xs rounded border border-dashed border-[var(--color-border)] text-[var(--color-muted)] hover:border-[var(--color-accent)] hover:text-[var(--color-accent)] transition-colors"
      >
        {t("create_inline")}
      </button>
      {definitions.length > 0 && (
        <div className="flex items-center gap-1">
          <span className="text-xs text-[var(--color-muted)]">{t("use_definition")}</span>
          <select
            value={refSel}
            onChange={(e) => setRefSel(e.target.value)}
            className="bg-[var(--color-canvas)] border border-[var(--color-border)] rounded px-1.5 py-0.5 text-xs text-[var(--color-text)] focus:outline-none focus:border-[var(--color-accent)]"
          >
            {definitions.map((d) => (
              <option key={d.name} value={d.name}>{d.name}</option>
            ))}
          </select>
          <button
            onClick={() => refSel && onAddRef(refSel)}
            disabled={!refSel}
            className="px-2 py-0.5 text-xs rounded border border-[var(--color-border)] text-[var(--color-muted)] hover:border-[var(--color-accent)] hover:text-[var(--color-accent)] disabled:opacity-40 transition-colors"
          >
            +
          </button>
        </div>
      )}
      {includes.length > 0 && (
        <div className="flex items-center gap-1">
          <span className="text-xs text-[var(--color-muted)]">{t("use_include")}</span>
          <select
            value={incSel}
            onChange={(e) => setIncSel(e.target.value)}
            className="bg-[var(--color-canvas)] border border-[var(--color-border)] rounded px-1.5 py-0.5 text-xs text-[var(--color-text)] focus:outline-none focus:border-[var(--color-accent)]"
          >
            {includes.map((inc) => (
              <option key={inc.ref} value={inc.ref}>{inc.ref}</option>
            ))}
          </select>
          <button
            onClick={() => incSel && onAddInclude(incSel)}
            disabled={!incSel}
            className="px-2 py-0.5 text-xs rounded border border-[var(--color-border)] text-[var(--color-muted)] hover:border-[var(--color-accent)] hover:text-[var(--color-accent)] disabled:opacity-40 transition-colors"
          >
            +
          </button>
        </div>
      )}
    </div>
  );
}

// ── Helpers ───────────────────────────────────────────────────────────────────

function stripKey({ _key: _k, ...rest }: DraftCmd): CommandEntry {
  void _k;
  return rest;
}
