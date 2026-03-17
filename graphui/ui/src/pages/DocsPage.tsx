import { useState, useEffect } from "react";
import { Link, useParams, useNavigate } from "react-router-dom";
import ReactMarkdown from "react-markdown";
import remarkGfm from "remark-gfm";
import { currentLanguage, setLanguage } from "../i18n";

// ── Markdown imports ─────────────────────────────────────────────────────────
// Docs live at <repo-root>/docs/{locale}/*.md, aliased as @docs

import enIntro from "@docs/en/intro.md?raw";
import enComparison from "@docs/en/comparison.md?raw";
import enGettingStarted from "@docs/en/getting-started.md?raw";
import enYamlSchema from "@docs/en/features-yaml-schema.md?raw";
import enTemplates from "@docs/en/features-templates.md?raw";
import enParallelization from "@docs/en/features-parallelization.md?raw";
import enGraphUi from "@docs/en/features-graph-ui.md?raw";
import enCaching from "@docs/en/features-caching.md?raw";
import enIncludes from "@docs/en/features-includes.md?raw";
import enMigration from "@docs/en/migration.md?raw";
import enUpgrading from "@docs/en/upgrading.md?raw";

import ruIntro from "@docs/ru/intro.md?raw";
import ruComparison from "@docs/ru/comparison.md?raw";
import ruGettingStarted from "@docs/ru/getting-started.md?raw";
import ruYamlSchema from "@docs/ru/features-yaml-schema.md?raw";
import ruTemplates from "@docs/ru/features-templates.md?raw";
import ruParallelization from "@docs/ru/features-parallelization.md?raw";
import ruGraphUi from "@docs/ru/features-graph-ui.md?raw";
import ruCaching from "@docs/ru/features-caching.md?raw";
import ruIncludes from "@docs/ru/features-includes.md?raw";
import ruMigration from "@docs/ru/migration.md?raw";
import ruUpgrading from "@docs/ru/upgrading.md?raw";

// ── Navigation structure ─────────────────────────────────────────────────────

interface NavItem {
  slug: string;
  label: { en: string; ru: string };
  children?: NavItem[];
}

const NAV: NavItem[] = [
  { slug: "intro", label: { en: "Introduction", ru: "Введение" } },
  { slug: "comparison", label: { en: "githops vs ...", ru: "githops vs ..." } },
  { slug: "getting-started", label: { en: "Getting Started", ru: "Начало работы" } },
  {
    slug: "features",
    label: { en: "Features", ru: "Функции" },
    children: [
      { slug: "yaml-schema", label: { en: "YAML Schema", ru: "YAML Schema" } },
      { slug: "templates", label: { en: "Templates", ru: "Шаблоны" } },
      { slug: "parallelization", label: { en: "Parallelization", ru: "Параллелизация" } },
      { slug: "graph-ui", label: { en: "Graph UI", ru: "Визуальный граф" } },
      { slug: "caching", label: { en: "Caching", ru: "Кэширование" } },
      { slug: "includes", label: { en: "External Includes", ru: "Внешние импорты" } },
    ],
  },
  { slug: "migration", label: { en: "Migration Guide", ru: "Миграция" } },
  { slug: "upgrading", label: { en: "Upgrading", ru: "Обновление" } },
];

// ── Content map ──────────────────────────────────────────────────────────────

const CONTENT: Record<string, { en: string; ru: string }> = {
  intro: { en: enIntro, ru: ruIntro },
  comparison: { en: enComparison, ru: ruComparison },
  "getting-started": { en: enGettingStarted, ru: ruGettingStarted },
  "yaml-schema": { en: enYamlSchema, ru: ruYamlSchema },
  templates: { en: enTemplates, ru: ruTemplates },
  parallelization: { en: enParallelization, ru: ruParallelization },
  "graph-ui": { en: enGraphUi, ru: ruGraphUi },
  caching: { en: enCaching, ru: ruCaching },
  "includes": { en: enIncludes, ru: ruIncludes },
  migration: { en: enMigration, ru: ruMigration },
  upgrading: { en: enUpgrading, ru: ruUpgrading },
};

const DEFAULT_SLUG = "intro";

// ── Sidebar ──────────────────────────────────────────────────────────────────

function SidebarItem({
  item,
  activeSlug,
  lang,
}: {
  item: NavItem;
  activeSlug: string;
  lang: "en" | "ru";
}) {
  const isActive = item.slug === activeSlug;
  const childActive = item.children?.some((c) => c.slug === activeSlug);
  const [open, setOpen] = useState(childActive ?? false);

  useEffect(() => {
    if (childActive) setOpen(true);
  }, [childActive]);

  if (item.children) {
    return (
      <li>
        <button
          onClick={() => setOpen((v) => !v)}
          className={`w-full text-left px-3 py-1.5 text-xs rounded transition-colors flex items-center justify-between cursor-pointer
            ${childActive ? "text-[var(--color-text)]" : "text-[var(--color-muted)] hover:text-[var(--color-text)]"}`}
        >
          <span>{item.label[lang]}</span>
          <span className="text-[10px]">{open ? "▾" : "▸"}</span>
        </button>
        {open && (
          <ul className="ml-3 border-l border-[var(--color-border)] pl-2 mt-0.5 space-y-0.5">
            {item.children.map((child) => (
              <SidebarItem key={child.slug} item={child} activeSlug={activeSlug} lang={lang} />
            ))}
          </ul>
        )}
      </li>
    );
  }

  return (
    <li>
      <Link
        to={`/docs/${item.slug}`}
        className={`block px-3 py-1.5 text-xs rounded transition-colors
          ${isActive
            ? "bg-[var(--color-surface)] text-[var(--color-accent)] font-semibold border-l-2 border-[var(--color-accent)] -ml-px pl-[calc(0.75rem-1px)]"
            : "text-[var(--color-muted)] hover:text-[var(--color-text)]"
          }`}
      >
        {item.label[lang]}
      </Link>
    </li>
  );
}

// ── Prose renderer ───────────────────────────────────────────────────────────

function Prose({ markdown }: { markdown: string }) {
  return (
    <div className="prose-docs">
      <ReactMarkdown remarkPlugins={[remarkGfm]}>{markdown}</ReactMarkdown>
    </div>
  );
}

// ── DocsPage ─────────────────────────────────────────────────────────────────

export function DocsPage() {
  const { slug } = useParams<{ slug?: string }>();
  const navigate = useNavigate();
  const [lang, setLang] = useState<"en" | "ru">(currentLanguage());

  const activeSlug = slug ?? DEFAULT_SLUG;
  const content = CONTENT[activeSlug];

  useEffect(() => {
    if (!slug) navigate(`/docs/${DEFAULT_SLUG}`, { replace: true });
  }, [slug, navigate]);

  function toggleLang() {
    const next = lang === "en" ? "ru" : "en";
    setLang(next);
    setLanguage(next);
  }

  return (
    <div className="flex flex-col h-screen bg-[var(--color-canvas)] text-[var(--color-text)] font-[var(--font-mono)]">
      {/* Header */}
      <header className="flex items-center gap-3 px-4 py-2 bg-[var(--color-surface)] border-b border-[var(--color-border)] shrink-0">
        <Link
          to="/"
          className="text-[var(--color-muted)] hover:text-[var(--color-text)] text-xs transition-colors"
        >
          ← githops graph
        </Link>
        <span className="text-[var(--color-border)]">/</span>
        <span className="text-[var(--color-accent)] font-bold text-sm tracking-tight">docs</span>
        <div className="ml-auto">
          <button
            onClick={toggleLang}
            className="text-xs text-[var(--color-muted)] hover:text-[var(--color-text)] transition-colors cursor-pointer px-2 py-1 border border-[var(--color-border)] rounded"
          >
            {lang === "en" ? "RU" : "EN"}
          </button>
        </div>
      </header>

      {/* Body */}
      <div className="flex flex-1 overflow-hidden">
        {/* Sidebar */}
        <nav className="w-52 shrink-0 overflow-y-auto border-r border-[var(--color-border)] bg-[var(--color-canvas)] py-4 px-2">
          <ul className="space-y-0.5">
            {NAV.map((item) => (
              <SidebarItem key={item.slug} item={item} activeSlug={activeSlug} lang={lang} />
            ))}
          </ul>
        </nav>

        {/* Content */}
        <main className="flex-1 overflow-y-auto px-10 py-8 max-w-4xl">
          {content ? (
            <Prose markdown={content[lang]} />
          ) : (
            <p className="text-[var(--color-muted)] text-sm">Page not found.</p>
          )}
        </main>
      </div>
    </div>
  );
}
