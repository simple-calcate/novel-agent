import { createContext, createElement, useContext, useEffect, useMemo, useState, type ReactNode } from "react";
import { type MessageKey } from "./catalogs";
import {
  applyDocumentLocale,
  getLocale,
  setLocale as commitLocale,
  subscribeLocale,
  t as translate,
} from "./runtime";
import { LOCALE_OPTIONS, type Locale, type MessageVars } from "./types";

interface I18nValue {
  locale: Locale;
  t: (key: MessageKey, vars?: MessageVars) => string;
  setLocale: (locale: Locale) => void;
  options: typeof LOCALE_OPTIONS;
}

const I18nContext = createContext<I18nValue | null>(null);

export function I18nProvider({ children }: { children: ReactNode }) {
  const [locale, setLocaleState] = useState<Locale>(() => getLocale());

  useEffect(() => {
    applyDocumentLocale(getLocale());
    return subscribeLocale(setLocaleState);
  }, []);

  const value = useMemo<I18nValue>(
    () => ({
      locale,
      t: translate,
      setLocale: commitLocale,
      options: LOCALE_OPTIONS,
    }),
    [locale],
  );

  return createElement(I18nContext.Provider, { value }, children);
}

export function useI18n(): I18nValue {
  const value = useContext(I18nContext);
  if (!value) {
    return {
      locale: getLocale(),
      t: translate,
      setLocale: commitLocale,
      options: LOCALE_OPTIONS,
    };
  }
  return value;
}
