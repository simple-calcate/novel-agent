import { en } from "./en";
import { zhCN } from "./zh-CN";
import type { DeepString, Locale, NestedKey } from "../types";

export type Messages = DeepString<typeof zhCN>;
export type MessageKey = NestedKey<typeof zhCN>;

export const catalogs: Record<Locale, Messages> = {
  "zh-CN": zhCN,
  en,
};

export { zhCN, en };
