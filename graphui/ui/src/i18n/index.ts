import en from "./en_US.json";
import ru from "./ru_RU.json";

const stored = localStorage.getItem("githops_lang");
const detected = navigator.language.toLowerCase().startsWith("ru") ? "ru" : "en";
const lang: "en" | "ru" =
  stored === "en" || stored === "ru" ? stored : (detected as "en" | "ru");

const msgs: Record<string, string> = (lang === "ru" ? ru : en) as Record<string, string>;

export function currentLanguage(): "en" | "ru" {
  return lang;
}

export function setLanguage(next: "en" | "ru"): void {
  localStorage.setItem("githops_lang", next);
  window.location.reload();
}

export function t(key: string, vars?: Record<string, string | number>): string {
  let s = msgs[key] ?? key;
  if (vars) {
    for (const [k, v] of Object.entries(vars)) {
      s = s.replace(`{${k}}`, String(v));
    }
  }
  return s;
}
