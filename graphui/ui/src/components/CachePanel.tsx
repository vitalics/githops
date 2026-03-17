import { useState } from "react";
import type { CacheStatus, WsResult } from "../types";
import { t } from "../i18n";
import { Button } from "./ui/button";
import { Switch } from "./ui/switch";
import { Table, TableHead, TableBody, TableRow, TableHeader, TableCell } from "./ui/table";

type SendFn = (method: string, params?: Record<string, unknown>) => Promise<WsResult>;

type Props = {
  cacheStatus: CacheStatus;
  send: SendFn;
};

export function CachePanel({ cacheStatus, send }: Props) {
  const [clearing, setClearing] = useState(false);
  const [toggling, setToggling] = useState(false);
  const [msg, setMsg] = useState<string | null>(null);

  async function handleToggleEnabled() {
    setToggling(true);
    setMsg(null);
    try {
      await send("cache.update", {
        enabled: !cacheStatus.enabled,
        dir: cacheStatus.dir,
      });
    } finally {
      setToggling(false);
    }
  }

  async function handleClear() {
    setClearing(true);
    setMsg(null);
    try {
      await send("cache.clear");
      setMsg(t("cache_cleared"));
    } finally {
      setClearing(false);
    }
  }

  return (
    <div className="p-5 max-w-4xl space-y-6">
      {/* ── Global config ── */}
      <section className="space-y-3">
        <h2 className="text-sm font-bold text-[var(--color-accent)]">{t("cache_config")}</h2>

        <div className="flex items-center gap-4 p-3 rounded-lg border border-[var(--color-border)] bg-[var(--color-surface)]">
          <div className="flex-1 space-y-0.5">
            <div className="flex items-center gap-2">
              <span
                className={`w-2 h-2 rounded-full shrink-0 ${
                  cacheStatus.enabled ? "bg-[var(--color-accent)]" : "bg-[var(--color-border)]"
                }`}
              />
              <span className="text-xs font-semibold text-[var(--color-text)]">
                {t("cache_enabled_global")}
              </span>
            </div>
            <p className="text-[10px] text-[var(--color-muted)] pl-4">
              {t("cache_dir")}: <code className="text-[var(--color-accent)]">{cacheStatus.dir}</code>
            </p>
          </div>
          <Switch
            checked={cacheStatus.enabled}
            onChange={handleToggleEnabled}
            disabled={toggling}
          />
        </div>
      </section>

      {/* ── Cache entries ── */}
      <section className="space-y-3">
        <div className="flex items-center justify-between">
          <h2 className="text-sm font-bold text-[var(--color-accent)]">
            {t("cache_entries")}{" "}
            <span className="font-normal text-[var(--color-muted)]">
              ({cacheStatus.entries.length})
            </span>
          </h2>
          {cacheStatus.entries.length > 0 && (
            <Button color="red" outline onClick={handleClear} disabled={clearing}>
              {clearing ? "…" : t("cache_clear")}
            </Button>
          )}
        </div>

        {msg && (
          <p className="text-xs text-[var(--color-accent)] bg-[var(--color-accent-dim)] border border-[var(--color-accent)] rounded px-3 py-2">
            {msg}
          </p>
        )}

        {cacheStatus.entries.length === 0 ? (
          <p className="text-xs text-[var(--color-muted)] py-4 text-center border border-dashed border-[var(--color-border)] rounded-lg">
            {t("cache_no_entries")}
          </p>
        ) : (
          <Table>
            <TableHead>
              <TableRow>
                <TableHeader>{t("cache_key")}</TableHeader>
                <TableHeader className="text-right w-32">{t("cache_age")}</TableHeader>
              </TableRow>
            </TableHead>
            <TableBody>
              {cacheStatus.entries
                .slice()
                .sort((a, b) => a.ageMs - b.ageMs)
                .map((entry) => (
                  <TableRow key={entry.key}>
                    <TableCell className="font-mono truncate max-w-xs">{entry.key}</TableCell>
                    <TableCell className="text-right text-[var(--color-muted)] whitespace-nowrap">
                      {formatAge(entry.ageMs)}
                    </TableCell>
                  </TableRow>
                ))}
            </TableBody>
          </Table>
        )}
      </section>
    </div>
  );
}

function formatAge(ms: number): string {
  const s = Math.floor(ms / 1000);
  if (s < 60) return `${s}s`;
  const m = Math.floor(s / 60);
  if (m < 60) return `${m}m`;
  const h = Math.floor(m / 60);
  if (h < 24) return `${h}h`;
  return `${Math.floor(h / 24)}d`;
}
