import { useState } from "react";
import type { UniqueCommand } from "../types";
import type { WsResult } from "../types";
import { t } from "../i18n";
import { Button } from "./ui/button";
import { Input } from "./ui/input";
import { Field, Label } from "./ui/fieldset";

type SendFn = (method: string, params?: Record<string, unknown>) => Promise<WsResult>;

// ── CommandsPanel ─────────────────────────────────────────────────────────────

type Props = {
  commands: UniqueCommand[];
  send: SendFn;
};

export function CommandsPanel({ commands, send }: Props) {
  const [filter, setFilter] = useState("");
  const [selected, setSelected] = useState<string | null>(null);
  const [dirty, setDirty] = useState(false);

  const filtered = commands.filter((c) =>
    !filter || c.name.toLowerCase().includes(filter.toLowerCase()),
  );

  function handleSelect(name: string) {
    if (selected === name) return;
    if (dirty && !window.confirm(t("confirm_discard"))) return;
    setDirty(false);
    setSelected(name);
  }

  const cmd = commands.find((c) => c.name === selected) ?? null;

  return (
    <div className="flex h-full">
      {/* Left: list */}
      <div className="w-52 shrink-0 border-r border-[var(--color-border)] overflow-y-auto flex flex-col">
        <div className="p-2 border-b border-[var(--color-border)]">
          <Input
            value={filter}
            onChange={(e) => setFilter(e.target.value)}
            placeholder={t("filter_commands")}
          />
        </div>
        <div className="flex-1 overflow-y-auto p-2 space-y-0.5">
          {commands.length === 0 ? (
            <p className="text-xs text-[var(--color-muted)] p-2">{t("no_commands")}</p>
          ) : filtered.length === 0 ? (
            <p className="text-xs text-[var(--color-muted)] p-2">{t("no_match")}</p>
          ) : (
            filtered.map((c) => (
              <button
                key={c.name}
                onClick={() => handleSelect(c.name)}
                className={`flex flex-col w-full text-left px-2 py-1.5 rounded text-xs transition-colors
                  ${selected === c.name
                    ? "bg-[var(--color-accent-dim)] text-[var(--color-accent)]"
                    : "text-[var(--color-text)] hover:bg-[var(--color-border)]"
                  }`}
              >
                <span className="font-semibold truncate">{c.name}</span>
                <span className="text-[10px] text-[var(--color-muted)] truncate">{c.run}</span>
              </button>
            ))
          )}
        </div>
      </div>

      {/* Right: editor */}
      <div className="flex-1 overflow-y-auto">
        {!cmd ? (
          <div className="flex items-center justify-center h-full text-[var(--color-muted)] text-sm whitespace-pre-line text-center">
            {t("select_command")}
          </div>
        ) : (
          <CommandEditor
            key={cmd.name}
            command={cmd}
            send={send}
            onDirtyChange={setDirty}
          />
        )}
      </div>
    </div>
  );
}

// ── CommandEditor ─────────────────────────────────────────────────────────────

function CommandEditor({
  command,
  send,
  onDirtyChange,
}: {
  command: UniqueCommand;
  send: SendFn;
  onDirtyChange: (dirty: boolean) => void;
}) {
  const [name, setName] = useState(command.name);
  const [run, setRun] = useState(command.run);
  const [saving, setSaving] = useState(false);

  const dirty = name !== command.name || run !== command.run;
  onDirtyChange(dirty);

  async function handleSave() {
    setSaving(true);
    try {
      await send("command.update", { oldName: command.name, name, run });
    } finally {
      setSaving(false);
    }
  }

  function handleCancel() {
    setName(command.name);
    setRun(command.run);
  }

  const renamed = name !== command.name;

  return (
    <div className="p-5 max-w-xl space-y-4">
      <div>
        <h2 className="text-sm font-bold text-[var(--color-accent)]">{command.name}</h2>
        {command.usedIn.length > 0 && (
          <p className="text-[10px] text-[var(--color-muted)] mt-0.5">
            {t("used_in")}: {command.usedIn.join(", ")}
          </p>
        )}
      </div>

      {command.usedIn.length > 1 && (
        <p className="text-[11px] text-[var(--color-muted)] bg-[var(--color-surface)] border border-[var(--color-border)] rounded px-3 py-2">
          {t("saves_all", { n: command.usedIn.length })}
        </p>
      )}
      {renamed && (
        <p className="text-[11px] text-yellow-400 bg-yellow-950/40 border border-yellow-900 rounded px-3 py-2">
          {t("rename_deps")}
        </p>
      )}

      <div className="space-y-3">
        <Field>
          <Label>{t("name")}</Label>
          <Input value={name} onChange={(e) => setName(e.target.value)} />
        </Field>
        <Field>
          <Label>{t("run")}</Label>
          <Input
            value={run}
            onChange={(e) => setRun(e.target.value)}
            className="font-mono"
          />
        </Field>
        <div className="flex items-center gap-1.5">
          <span className="text-[9px] uppercase tracking-widest text-[var(--color-muted)]">{t("test_only")}:</span>
          <span className={`text-xs ${command.test ? "text-[var(--color-accent)]" : "text-[var(--color-muted)]"}`}>
            {command.test ? "yes" : "no"}
          </span>
        </div>
      </div>

      <div className="flex gap-2 pt-2 border-t border-[var(--color-border)]">
        <Button onClick={handleSave} disabled={saving || !dirty}>{t("save")}</Button>
        <Button outline onClick={handleCancel} disabled={!dirty}>{t("cancel")}</Button>
      </div>
    </div>
  );
}
