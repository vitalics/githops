import { useState } from "react";
import { Link } from "react-router-dom";
import { FlowGraph } from "./components/FlowGraph";
import { HookPanel } from "./components/HookPanel";
import { CommandsPanel } from "./components/CommandsPanel";
import { DefinitionsPanel } from "./components/DefinitionsPanel";
import { CachePanel } from "./components/CachePanel";
import { useGithopsWs } from "./hooks/useGithopsWs";
import { t, currentLanguage, setLanguage } from "./i18n";
import { Button } from "./components/ui/button";

type Tab = "hooks" | "commands" | "definitions" | "flow" | "cache";

const TABS: Tab[] = ["hooks", "commands", "definitions", "flow", "cache"];

const TAB_LABELS: Record<Tab, string> = {
  hooks: "hooks",
  commands: "commands",
  definitions: "definitions",
  flow: "flow",
  cache: "cache",
};

export default function App() {
  const { appState, connected, send } = useGithopsWs();
  const [tab, setTab] = useState<Tab>("hooks");
  const [syncing, setSyncing] = useState(false);
  const [syncMsg, setSyncMsg] = useState<string | null>(null);
  const [defNavTarget, setDefNavTarget] = useState<string | null>(null);

  async function handleSync() {
    setSyncing(true);
    setSyncMsg(null);
    try {
      const r = await send("sync");
      setSyncMsg((r.message as string | undefined) ?? t("sync_ok"));
    } catch (e) {
      setSyncMsg("Sync failed: " + (e instanceof Error ? e.message : String(e)));
    } finally {
      setSyncing(false);
    }
  }

  function navigateToDefs(defName: string) {
    setDefNavTarget(defName);
    setTab("definitions");
  }

  const statusText = connected
    ? appState?.configExists
      ? t("config_loaded")
      : t("no_config")
    : t("connecting");

  return (
    <div className="flex flex-col h-screen bg-[var(--color-canvas)] text-[var(--color-text)] font-[var(--font-mono)]">
      {/* ── Header ── */}
      <header className="flex items-center gap-3 px-4 py-2 bg-[var(--color-surface)] border-b border-[var(--color-border)] shrink-0">
        <span className="text-[var(--color-accent)] font-bold text-sm tracking-tight">
          githops graph
        </span>
        <Link
          to="/docs"
          className="text-xs text-[var(--color-muted)] hover:text-[var(--color-text)] transition-colors mr-auto"
        >
          docs
        </Link>

        {syncMsg && (
          <span className="text-xs text-[var(--color-muted)]">{syncMsg}</span>
        )}

        <Button
          outline
          onClick={handleSync}
          disabled={syncing || !connected}
        >
          {syncing ? t("sync") + "\u2026" : t("sync")}
        </Button>

        <Button
          plain
          onClick={() => setLanguage(currentLanguage() === "en" ? "ru" : "en")}
        >
          {currentLanguage() === "en" ? "RU" : "EN"}
        </Button>

        <div className="flex items-center gap-1.5">
          <span
            className={`w-2 h-2 rounded-full ${connected ? "bg-[var(--color-accent)]" : "bg-red-500"}`}
          />
          <span className="text-xs text-[var(--color-muted)]">{statusText}</span>
        </div>
      </header>

      {/* ── Tab bar ── */}
      <nav className="flex gap-0.5 px-3 bg-[var(--color-surface)] border-b border-[var(--color-border)] shrink-0">
        {TABS.map((tabId) => (
          <button
            key={tabId}
            onClick={() => setTab(tabId)}
            className={`px-4 py-2 text-xs capitalize transition-colors border-b-2 -mb-px cursor-pointer
              ${
                tab === tabId
                  ? "border-[var(--color-accent)] text-[var(--color-accent)] font-semibold"
                  : "border-transparent text-[var(--color-muted)] hover:text-[var(--color-text)]"
              }`}
          >
            {t(TAB_LABELS[tabId])}
          </button>
        ))}
      </nav>

      {/* ── Content ── */}
      <main className="flex-1 overflow-hidden">
        {!appState ? (
          <div className="flex items-center justify-center h-full text-[var(--color-muted)] text-sm">
            {t("connecting")}…
          </div>
        ) : tab === "hooks" ? (
          <div className="h-full overflow-hidden">
            <HookPanel
              hooks={appState.hooks}
              definitions={appState.definitions}
              send={send}
              onNavigateToDefs={navigateToDefs}
            />
          </div>
        ) : tab === "commands" ? (
          <div className="h-full overflow-hidden">
            <CommandsPanel commands={appState.commands} send={send} />
          </div>
        ) : tab === "definitions" ? (
          <div className="h-full overflow-hidden">
            <DefinitionsPanel
              definitions={appState.definitions}
              send={send}
              navTarget={defNavTarget}
            />
          </div>
        ) : tab === "flow" ? (
          <div className="h-full">
            <FlowGraph hooks={appState.hooks} definitions={appState.definitions} send={send} />
          </div>
        ) : (
          <div className="h-full overflow-y-auto">
            <CachePanel
              cacheStatus={appState.cacheStatus ?? { enabled: false, dir: ".githops/cache", entries: [] }}
              send={send}
            />
          </div>
        )}
      </main>
    </div>
  );
}
