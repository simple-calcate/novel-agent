import { catalogs } from "./catalogs";
import {
  DEFAULT_LOCALE,
  LOCALES,
  LOCALE_STORAGE_KEY,
  type Locale,
  type MessageVars,
} from "./types";
import type { MessageKey, Messages } from "./catalogs";

type LocaleListener = (locale: Locale) => void;

const listeners = new Set<LocaleListener>();
let currentLocale: Locale = DEFAULT_LOCALE;
let detected = false;

function isLocale(value: string | null | undefined): value is Locale {
  return LOCALES.includes(value as Locale);
}

function readStoredLocale(): Locale | null {
  if (typeof localStorage === "undefined") return null;
  try {
    const stored = localStorage.getItem(LOCALE_STORAGE_KEY);
    return isLocale(stored) ? stored : null;
  } catch {
    return null;
  }
}

function persistLocale(locale: Locale): void {
  if (typeof localStorage === "undefined") return;
  try {
    localStorage.setItem(LOCALE_STORAGE_KEY, locale);
  } catch {
    // Ignore quota / private-mode failures; in-memory locale still works.
  }
}

function navigatorLocale(): Locale | null {
  if (typeof navigator === "undefined") return null;
  const candidates = [navigator.language, ...(navigator.languages ?? [])];
  for (const item of candidates) {
    const lower = item?.toLowerCase() ?? "";
    if (lower.startsWith("zh")) return "zh-CN";
    if (lower.startsWith("en")) return "en";
  }
  return null;
}

function lookup(tree: Messages, key: MessageKey): string | undefined {
  let node: unknown = tree;
  for (const part of key.split(".")) {
    if (!node || typeof node !== "object" || !(part in node)) return undefined;
    node = (node as Record<string, unknown>)[part];
  }
  return typeof node === "string" ? node : undefined;
}

export function interpolate(template: string, vars?: MessageVars): string {
  if (!vars) return template;
  return template.replace(/\{(\w+)\}/g, (whole, name: string) =>
    Object.prototype.hasOwnProperty.call(vars, name) ? String(vars[name]) : whole,
  );
}

export function detectLocale(): Locale {
  return readStoredLocale() ?? navigatorLocale() ?? DEFAULT_LOCALE;
}

export function getLocale(): Locale {
  if (!detected) {
    currentLocale = detectLocale();
    detected = true;
  }
  return currentLocale;
}

export function collatorLocale(locale: Locale = getLocale()): string {
  return locale === "zh-CN" ? "zh-CN" : "en";
}

export function applyDocumentLocale(locale: Locale = getLocale()): void {
  if (typeof document === "undefined") return;
  document.documentElement.lang = locale;
  document.title = t("meta.title");
}

export function setLocale(locale: Locale): void {
  if (currentLocale === locale && detected) {
    persistLocale(locale);
    applyDocumentLocale(locale);
    return;
  }
  currentLocale = locale;
  detected = true;
  persistLocale(locale);
  applyDocumentLocale(locale);
  listeners.forEach((listener) => listener(locale));
}

export function subscribeLocale(listener: LocaleListener): () => void {
  listeners.add(listener);
  return () => listeners.delete(listener);
}

export function t(key: MessageKey, vars?: MessageVars): string {
  const locale = getLocale();
  const text = lookup(catalogs[locale], key) ?? lookup(catalogs[DEFAULT_LOCALE], key) ?? key;
  return interpolate(text, vars);
}

export function joinList(items: string[]): string {
  return items.join(t("common.listJoin"));
}

export function compareText(left: string, right: string): number {
  return left.localeCompare(right, collatorLocale());
}

export function formatDiffSummary(inserted: number, deleted: number, identical = false): string {
  if (identical || (inserted === 0 && deleted === 0)) return t("history.unchanged");
  if (deleted === 0) return t("history.inserted", { count: inserted });
  if (inserted === 0) return t("history.deleted", { count: deleted });
  return t("history.both", { inserted, deleted });
}

export function catalogKeys(tree: object, prefix = ""): string[] {
  const keys: string[] = [];
  for (const [name, value] of Object.entries(tree)) {
    const path = prefix ? `${prefix}.${name}` : name;
    if (typeof value === "string") keys.push(path);
    else if (value && typeof value === "object") keys.push(...catalogKeys(value, path));
  }
  return keys;
}

/** Tests: pin a locale without writing localStorage (still notifies subscribers). */
export function resetLocaleForTests(locale: Locale = DEFAULT_LOCALE): void {
  currentLocale = locale;
  detected = true;
  listeners.forEach((listener) => listener(locale));
}
