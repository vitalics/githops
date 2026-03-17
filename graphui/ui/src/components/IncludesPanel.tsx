import { useState } from "react";
import type { IncludeSource, WsResult } from "../types";
import { t } from "../i18n";
import { Button } from "./ui/button";

type SendFn = (method: string, params?: Record<string, unknown>) => Promise<WsResult>;

type SourceType = "local" | "remote" | "git";
type FileType = "json" | "toml" | "yaml";

interface EditState {
  oldRef: string;
  source: SourceType;
  ref: string;
  path: string;   // local
  url: string;    // remote + git
  rev: string;    // git
  file: string;   // git
  type: FileType;
}

function emptyEdit(source: SourceType = "local"): EditState {
  return { oldRef: "", source, ref: "", path: "", url: "", rev: "main", file: "", type: "yaml" };
}

const SOURCE_BADGE: Record<SourceType, { label: string; color: string }> = {
  local:  { label: "local",  color: "#00d4aa" },
  remote: { label: "remote", color: "#6496ff" },
  git:    { label: "git",    color: "#f97316" },
};

type Props = {
  includes: IncludeSource[];
  send: SendFn;
};

export function IncludesPanel({ includes, send }: Props) {
  const [selected, setSelected] = useState<string | null>(null);
  const [editing, setEditing] = useState<EditState | null>(null);

  function startAdd(source: SourceType = "local") {
    setEditing(emptyEdit(source));
    setSelected(null);
  }

  function startEdit(inc: IncludeSource) {
    setSelected(inc.ref);
    setEditing({
      oldRef:  inc.ref,
      source:  inc.source,
      ref:     inc.ref,
      path:    inc.path ?? "",
      url:     inc.url  ?? "",
      rev:     inc.rev  ?? "main",
      file:    inc.file ?? "",
      type:    inc.type ?? "yaml",
    });
  }

  function cancelEdit() {
    setEditing(null);
    setSelected(null);
  }

  async function saveEdit() {
    if (!editing || !editing.ref.trim()) return;
    await send("include.update", {
      oldRef:  editing.oldRef,
      source:  editing.source,
      ref:     editing.ref.trim(),
      path:    editing.path.trim(),
      url:     editing.url.trim(),
      rev:     editing.rev.trim(),
      file:    editing.file.trim(),
      type:    editing.type,
    });
    setEditing(null);
    setSelected(null);
  }

  async function deleteInclude(ref: string) {
    await send("include.delete", { ref });
    if (selected === ref) { setSelected(null); setEditing(null); }
  }

  function isSaveDisabled(): boolean {
    if (!editing || !editing.ref.trim()) return true;
    if (editing.source === "local"  && !editing.path.trim()) return true;
    if (editing.source === "remote" && !editing.url.trim())  return true;
    if (editing.source === "git"    && (!editing.url.trim() || !editing.rev.trim() || !editing.file.trim())) return true;
    return false;
  }

  const labelStyle: React.CSSProperties = {
    fontSize: 10,
    color: "var(--color-muted)",
    textTransform: "uppercase",
    letterSpacing: 1,
    fontFamily: '"SF Mono","Fira Code",monospace',
    display: "block",
    marginBottom: 4,
  };

  const inputStyle: React.CSSProperties = {
    background: "rgba(255,255,255,0.04)",
    border: "1px solid rgba(0,212,170,0.2)",
    borderRadius: 4,
    padding: "5px 9px",
    color: "#e6edf3",
    fontSize: 12,
    fontFamily: '"SF Mono","Fira Code",monospace',
    outline: "none",
    width: "100%",
    boxSizing: "border-box",
  };

  return (
    <div className="flex h-full">
      {/* ── List ── */}
      <div style={{ width: 260, borderRight: "1px solid var(--color-border)", display: "flex", flexDirection: "column", flexShrink: 0 }}>
        <div style={{ padding: "10px 12px", borderBottom: "1px solid var(--color-border)", display: "flex", alignItems: "center", justifyContent: "space-between", gap: 6 }}>
          <span style={{ fontSize: 11, color: "var(--color-muted)", fontFamily: '"SF Mono","Fira Code",monospace' }}>
            {t("includes")}
          </span>
          <div style={{ display: "flex", gap: 4 }}>
            {(["local", "remote", "git"] as SourceType[]).map((src) => (
              <button
                key={src}
                onClick={() => startAdd(src)}
                style={{
                  fontSize: 10,
                  padding: "2px 6px",
                  borderRadius: 4,
                  border: `1px solid ${SOURCE_BADGE[src].color}55`,
                  background: `${SOURCE_BADGE[src].color}10`,
                  color: SOURCE_BADGE[src].color,
                  cursor: "pointer",
                  fontFamily: '"SF Mono","Fira Code",monospace',
                }}
              >
                + {src}
              </button>
            ))}
          </div>
        </div>

        <div style={{ flex: 1, overflowY: "auto" }}>
          {includes.length === 0 && !editing && (
            <div style={{ padding: "20px 16px", fontSize: 12, color: "var(--color-muted)", fontFamily: '"SF Mono","Fira Code",monospace' }}>
              {t("no_includes")}
            </div>
          )}
          {includes.map((inc) => {
            const badge = SOURCE_BADGE[inc.source] ?? SOURCE_BADGE.local;
            return (
              <div
                key={inc.ref}
                onClick={() => startEdit(inc)}
                style={{
                  padding: "10px 14px",
                  cursor: "pointer",
                  borderBottom: "1px solid rgba(0,212,170,0.06)",
                  background: selected === inc.ref ? "rgba(0,212,170,0.06)" : "transparent",
                  transition: "background 0.1s",
                }}
              >
                <div style={{ display: "flex", alignItems: "center", gap: 6, marginBottom: 2 }}>
                  <span style={{ fontSize: 12, color: badge.color, fontFamily: '"SF Mono","Fira Code",monospace', fontWeight: 600 }}>
                    {inc.ref}
                  </span>
                  <span style={{ fontSize: 9, padding: "1px 4px", background: `${badge.color}15`, border: `1px solid ${badge.color}40`, borderRadius: 3, color: badge.color, fontFamily: '"SF Mono","Fira Code",monospace' }}>
                    {badge.label}
                  </span>
                </div>
                <div style={{ fontSize: 11, color: "var(--color-muted)", fontFamily: '"SF Mono","Fira Code",monospace', overflow: "hidden", textOverflow: "ellipsis", whiteSpace: "nowrap" }}>
                  {inc.source === "local"  && inc.path}
                  {inc.source === "remote" && inc.url}
                  {inc.source === "git"    && `${inc.url} @ ${inc.rev}`}
                  <span style={{ marginLeft: 6, padding: "1px 4px", background: "rgba(255,255,255,0.06)", borderRadius: 3, fontSize: 10 }}>
                    {inc.type}
                  </span>
                </div>
              </div>
            );
          })}
        </div>
      </div>

      {/* ── Editor ── */}
      <div style={{ flex: 1, padding: "20px 24px", overflowY: "auto" }}>
        {!editing ? (
          <div style={{ color: "var(--color-muted)", fontSize: 12, fontFamily: '"SF Mono","Fira Code",monospace', marginTop: 40, textAlign: "center" }}>
            {t("select_include")}
          </div>
        ) : (
          <div style={{ maxWidth: 460, display: "flex", flexDirection: "column", gap: 14 }}>
            {/* Source type selector */}
            <div>
              <label style={labelStyle}>{t("include_source")}</label>
              <div style={{ display: "flex", gap: 6 }}>
                {(["local", "remote", "git"] as SourceType[]).map((src) => {
                  const badge = SOURCE_BADGE[src];
                  const active = editing.source === src;
                  return (
                    <button
                      key={src}
                      onClick={() => setEditing({ ...emptyEdit(src), oldRef: editing.oldRef, ref: editing.ref })}
                      style={{
                        fontSize: 11,
                        padding: "4px 10px",
                        borderRadius: 5,
                        border: `1.5px solid ${active ? badge.color : badge.color + "44"}`,
                        background: active ? `${badge.color}18` : "transparent",
                        color: active ? badge.color : badge.color + "88",
                        cursor: "pointer",
                        fontFamily: '"SF Mono","Fira Code",monospace',
                        transition: "all 0.1s",
                      }}
                    >
                      {src}
                    </button>
                  );
                })}
              </div>
            </div>

            {/* Ref name (always shown) */}
            <div>
              <label style={labelStyle}>{t("include_ref")}</label>
              <input
                value={editing.ref}
                onChange={(e) => setEditing({ ...editing, ref: e.target.value })}
                placeholder="packagejson"
                style={inputStyle}
              />
            </div>

            {/* local: path */}
            {editing.source === "local" && (
              <div>
                <label style={labelStyle}>{t("include_path")}</label>
                <input
                  value={editing.path}
                  onChange={(e) => setEditing({ ...editing, path: e.target.value })}
                  placeholder="package.json"
                  style={inputStyle}
                />
              </div>
            )}

            {/* remote / git: url */}
            {(editing.source === "remote" || editing.source === "git") && (
              <div>
                <label style={labelStyle}>{t("include_url")}</label>
                <input
                  value={editing.url}
                  onChange={(e) => setEditing({ ...editing, url: e.target.value })}
                  placeholder={editing.source === "remote"
                    ? "https://example.com/scripts.yaml"
                    : "https://github.com/org/repo.git"}
                  style={inputStyle}
                />
              </div>
            )}

            {/* git: rev */}
            {editing.source === "git" && (
              <div>
                <label style={labelStyle}>{t("include_rev")}</label>
                <input
                  value={editing.rev}
                  onChange={(e) => setEditing({ ...editing, rev: e.target.value })}
                  placeholder="main"
                  style={inputStyle}
                />
              </div>
            )}

            {/* git: file */}
            {editing.source === "git" && (
              <div>
                <label style={labelStyle}>{t("include_file")}</label>
                <input
                  value={editing.file}
                  onChange={(e) => setEditing({ ...editing, file: e.target.value })}
                  placeholder="ci/scripts.yaml"
                  style={inputStyle}
                />
              </div>
            )}

            {/* type (always shown) */}
            <div>
              <label style={labelStyle}>{t("include_type")}</label>
              <select
                value={editing.type}
                onChange={(e) => setEditing({ ...editing, type: e.target.value as FileType })}
                style={{ ...inputStyle, cursor: "pointer" }}
              >
                <option value="yaml">YAML</option>
                <option value="json">JSON</option>
                <option value="toml">TOML</option>
              </select>
            </div>

            {/* Actions */}
            <div style={{ display: "flex", gap: 8, marginTop: 4 }}>
              <Button outline onClick={saveEdit} disabled={isSaveDisabled()}>
                {t("save")}
              </Button>
              <Button plain onClick={cancelEdit}>{t("cancel")}</Button>
              {editing.oldRef && (
                <Button
                  plain
                  onClick={() => deleteInclude(editing.oldRef)}
                  style={{ marginLeft: "auto", color: "#f87171" }}
                >
                  {t("delete")}
                </Button>
              )}
            </div>
          </div>
        )}
      </div>
    </div>
  );
}
