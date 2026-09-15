import { beforeEach } from "vitest";
import { DEFAULT_LOCALE, resetLocaleForTests } from "./index";

beforeEach(() => {
  resetLocaleForTests(DEFAULT_LOCALE);
});
