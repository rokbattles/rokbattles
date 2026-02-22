"use client";

import { defaultLocale, isLocale, type Locale, languageCookieName } from "@/i18n/config";
import { canUseDom } from "@/lib/util/can-use-dom";

const getLocaleFromCookie = (): Locale | undefined => {
  if (!canUseDom) return undefined;

  const entry = document.cookie
    .split("; ")
    .find((cookie) => cookie.startsWith(`${languageCookieName}=`));
  if (!entry) return undefined;

  const value = decodeURIComponent(entry.split("=").slice(1).join("="));
  return isLocale(value) ? value : undefined;
};

export function resolveLocale(locale?: string): Locale {
  if (locale && isLocale(locale)) {
    return locale;
  }

  return getLocaleFromCookie() ?? defaultLocale;
}
