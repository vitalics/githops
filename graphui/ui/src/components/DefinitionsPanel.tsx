import { useState, useEffect, useRef } from "react";
import type { DefCommand, DefinitionEntry } from "../types";
import type { WsResult } from "../types";
import { t } from "../i18n";
import { Button } from "./ui/button";
import { Input } from "./ui/input";
import { Checkbox } from "./ui/checkbox";
import { Field, Label } from "./ui/fieldset";

type SendFn = (method: string, params?: Record<string, unknown>) => Promise<WsResult>;

type DraftDefCmd = DefCommand & { _key: string };

// ── DefinitionsPanel ──────────────────────────────────────────────────────────

type Props = {
  definitions: DefinitionEntry[];
  send: SendFn;
  navTarget: string | null;
};

export function DefinitionsPanel({ definitions, send, navTarget }: Props) {
  const [selected, setSelected] = useState<string | null>(null);
  const [isNew, setIsNew] = useState(false);
  const isDirtyRef = useRef(false);

  // Navigation from hook panel "Edit in Definitions"
  useEffect(() => {
    if (navTarget) setSelected(navTarget);
  }, [navTarget]);

  function handleSelect(name: string) {
    if (selected === name && !isNew) return;
    if (isDirtyRef.current && !window.confirm(t("confirm_discard"))) return;
    isDirtyRef.current = false;
    setIsNew(false);
    setSelected(name);
  }

  function handleNew() {
    if (isDirtyRef.current && !window.confirm(t("confirm_discard"))) return;
    isDirtyRef.current = false;
    setIsNew(true);
    setSelected(null);
  }

  const def = !isNew ? (definitions.find((d) => d.name === selected) ?? null) : null;

  return (
    <div className="flex h-full">
      {/* Left: list */}
      <div className="w-52 shrink-0 border-r border-[var(--color-border)] overflow-y-auto p-2 space-y-0.5">
        <button
          onClick={handleNew}
          className="flex items-center gap-1.5 w-full text-left px-2 py-1.5 rounded text-xs text-[var(--color-accent)] hover:bg-[var(--color-border)] transition-colors mb-1"
        >
          {t("add_definition")}
        </button>

        {definitions.length === 0 && (
          <p className="text-xs text-[var(--color-muted)] px-2 py-1">{t("no_commands")}</p>
        )}
        {definitions.map((d) => (
          <button
            key={d.name}
            onClick={() => handleSelect(d.name)}
            className={`flex items-center justify-between w-full text-left px-2 py-1.5 rounded text-xs transition-colors
              ${selected === d.name && !isNew
                ? "bg-[var(--color-accent-dim)] text-[var(--color-accent)]"
                : "text-[var(--color-text)] hover:bg-[var(--color-border)]"
              }`}
          >
            <span className="truncate">{d.name}</span>
            <span className="text-[9px] text-[var(--color-muted)] shrink-0 ml-1">
              {d.type === "list" ? `${d.commands.length}×` : "1"}
            </span>
          </button>
        ))}
      </div>

      {/* Right: editor */}
      <div className="flex-1 overflow-y-auto">
        {isNew ? (
          <DefEditor
            key="__new__"
            entry={null}
            send={send}
            onDirtyChange={(d) => { isDirtyRef.current = d; }}
            onSaved={(name) => { setIsNew(false); setSelected(name); isDirtyRef.current = false; }}
            onCancelled={() => { setIsNew(false); isDirtyRef.current = false; }}
          />
        ) : !def ? (
          <div className="flex items-center justify-center h-full text-[var(--color-muted)] text-sm">
            {t("select_definition")}
          </div>
        ) : (
          <DefEditor
            key={def.name}
            entry={def}
            send={send}
            onDirtyChange={(d) => { isDirtyRef.current = d; }}
            onSaved={(name) => { setSelected(name); isDirtyRef.current = false; }}
            onCancelled={() => { isDirtyRef.current = false; }}
          />
        )}
      </div>
    </div>
  );
}

// ── DefEditor ─────────────────────────────────────────────────────────────────

function DefEditor({
  entry,
  send,
  onDirtyChange,
  onSaved,
  onCancelled,
}: {
  entry: DefinitionEntry | null;
  send: SendFn;
  onDirtyChange: (dirty: boolean) => void;
  onSaved: (name: string) => void;
  onCancelled: () => void;
}) {
  const keyRef = useRef(0);
  const isNew = entry === null;

  const [defName, setDefName] = useState(entry?.name ?? "");
  const [type, setType] = useState<"single" | "list">(entry?.type ?? "single");
  const [cmds, setCmds] = useState<DraftDefCmd[]>(() =>
    entry
      ? entry.commands.map((c) => ({ ...c, _key: String(keyRef.current++) }))
      : [blankCmd(keyRef)],
  );
  const [saving, setSaving] = useState(false);

  const origJson = JSON.stringify(entry);
  const draftJson = JSON.stringify({ name: defName, type, commands: cmds.map(stripKey) });
  const dirty = isNew
    ? defName.trim() !== "" || cmds.some((c) => c.name || c.run)
    : draftJson !== origJson;

  const onDirtyRef = useRef(onDirtyChange);
  onDirtyRef.current = onDirtyChange;
  useEffect(() => { onDirtyRef.current(dirty); }, [dirty]);

  function handleTypeChange(next: "single" | "list") {
    if (next === "list" && type === "single") {
      setType("list");
    } else if (next === "single" && type === "list") {
      setCmds((prev) => prev.slice(0, 1));
      setType("single");
    }
  }

  function addCmd() {
    setCmds((prev) => [...prev, blankCmd(keyRef)]);
  }
  function deleteCmd(key: string) {
    if (type === "single") return;
    setCmds((prev) => prev.filter((c) => c._key !== key));
  }
  function updateCmd(key: string, patch: Partial<DraftDefCmd>) {
    setCmds((prev) => prev.map((c) => (c._key === key ? { ...c, ...patch } : c)));
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

  async function handleSave() {
    const name = defName.trim();
    if (!name) return;
    setSaving(true);
    try {
      await send("definition.update", {
        name,
        oldName: isNew ? "" : (entry?.name ?? ""),
        defType: type,
        commands: cmds.map(stripKey),
      });
      onSaved(name);
    } finally {
      setSaving(false);
    }
  }

  async function handleDelete() {
    if (!entry || !window.confirm(t("confirm_discard"))) return;
    await send("definition.delete", { name: entry.name });
    onCancelled();
  }

  function handleCancel() {
    if (isNew) {
      onCancelled();
    } else if (entry) {
      setDefName(entry.name);
      setType(entry.type);
      setCmds(entry.commands.map((c) => ({ ...c, _key: String(keyRef.current++) })));
      onCancelled();
    }
  }

  const cmdNames = cmds.map((c) => c.name).filter(Boolean);

  return (
    <div className="p-5 max-w-4xl space-y-4">
      {/* Name */}
      <Field>
        <Label>{t("def_name")}</Label>
        <Input
          value={defName}
          onChange={(e) => setDefName(e.target.value)}
          placeholder="my-definition"
        />
      </Field>

      {/* Type */}
      <div className="flex items-center gap-3">
        <span className="text-[9px] uppercase tracking-widest text-[var(--color-muted)]">{t("type_label")}:</span>
        {(["single", "list"] as const).map((tp) => (
          <label key={tp} className="flex items-center gap-1.5 text-xs text-[var(--color-muted)] cursor-pointer">
            <input
              type="radio"
              name="defType"
              value={tp}
              checked={type === tp}
              onChange={() => handleTypeChange(tp)}
              className="accent-[var(--color-accent)]"
            />
            {tp === "single" ? t("def_type_single") : t("def_type_list")}
          </label>
        ))}
      </div>

      {/* Commands */}
      <div className="space-y-2">
        {cmds.map((cmd, i) => (
          <DefCmdCard
            key={cmd._key}
            cmd={cmd}
            index={i}
            total={cmds.length}
            canDelete={type === "list"}
            cmdNames={cmdNames}
            onChange={(patch) => updateCmd(cmd._key, patch)}
            onDelete={() => deleteCmd(cmd._key)}
            onMove={(dir) => moveCmd(cmd._key, dir)}
          />
        ))}
        {type === "list" && (
          <button
            onClick={addCmd}
            className="px-2 py-1 text-xs rounded border border-dashed border-[var(--color-border)] text-[var(--color-muted)] hover:border-[var(--color-accent)] hover:text-[var(--color-accent)] transition-colors"
          >
            {t("add_command")}
          </button>
        )}
      </div>

      {/* Footer */}
      <div className="flex gap-2 pt-3 border-t border-[var(--color-border)]">
        <Button
          onClick={handleSave}
          disabled={saving || (!isNew && !dirty) || defName.trim() === ""}
        >
          {t("save")}
        </Button>
        <Button outline onClick={handleCancel} disabled={!dirty && !isNew}>
          {t("cancel")}
        </Button>
        {!isNew && (
          <Button color="red" outline onClick={handleDelete} className="ml-auto">
            {t("delete")}
          </Button>
        )}
      </div>
    </div>
  );
}

// ── DefCmdCard ────────────────────────────────────────────────────────────────

function DefCmdCard({
  cmd,
  index,
  total,
  canDelete,
  cmdNames,
  onChange,
  onDelete,
  onMove,
}: {
  cmd: DraftDefCmd;
  index: number;
  total: number;
  canDelete: boolean;
  cmdNames: string[];
  onChange: (patch: Partial<DraftDefCmd>) => void;
  onDelete: () => void;
  onMove: (dir: -1 | 1) => void;
}) {
  const otherNames = cmdNames.filter((n) => n !== cmd.name);

  return (
    <div className="border border-[var(--color-border)] rounded-lg p-3 bg-[var(--color-surface)] space-y-2">
      {canDelete && (
        <div className="flex items-center gap-1 justify-end -mt-1 -mr-1">
          <button
            onClick={() => onMove(-1)}
            disabled={index === 0}
            className="text-[var(--color-muted)] hover:text-[var(--color-text)] disabled:opacity-25 text-xs w-5 h-5"
          >▲</button>
          <button
            onClick={() => onMove(1)}
            disabled={index === total - 1}
            className="text-[var(--color-muted)] hover:text-[var(--color-text)] disabled:opacity-25 text-xs w-5 h-5"
          >▼</button>
          <button onClick={onDelete} className="text-[var(--color-muted)] hover:text-red-400 text-xs w-5 h-5">
            ×
          </button>
        </div>
      )}
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
    </div>
  );
}

// ── Helpers ───────────────────────────────────────────────────────────────────

function blankCmd(keyRef: React.MutableRefObject<number>): DraftDefCmd {
  return { _key: String(keyRef.current++), name: "", run: "", depends: [], env: {}, test: false };
}

function stripKey({ _key: _k, ...rest }: DraftDefCmd): DefCommand {
  void _k;
  return rest;
}
