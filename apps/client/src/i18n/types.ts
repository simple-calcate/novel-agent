export const LOCALES = ["zh-CN", "en"] as const;

export type Locale = (typeof LOCALES)[number];

export const DEFAULT_LOCALE: Locale = "zh-CN";

export const LOCALE_STORAGE_KEY = "moshu.locale";

export const LOCALE_OPTIONS: Array<{ id: Locale; nativeLabel: string }> = [
  { id: "zh-CN", nativeLabel: "简体中文" },
  { id: "en", nativeLabel: "English" },
];

export type MessageVars = Record<string, string | number>;

export type DeepString<T> = {
  [K in keyof T]: T[K] extends string ? string : DeepString<T[K]>;
};

export type NestedKey<T, Prefix extends string = ""> = {
  [K in keyof T & string]: T[K] extends string
    ? Prefix extends ""
      ? K
      : `${Prefix}.${K}`
    : NestedKey<T[K], Prefix extends "" ? K : `${Prefix}.${K}`>;
}[keyof T & string];
