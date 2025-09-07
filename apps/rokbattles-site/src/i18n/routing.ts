import { defineRouting } from "next-intl/routing";

export const routing = defineRouting({
  locales: ["en", "es", "kr"],
  defaultLocale: "en",
  localePrefix: "as-needed",
});
