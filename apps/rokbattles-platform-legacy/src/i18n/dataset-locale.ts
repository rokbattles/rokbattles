"use client";

import { defaultLocale, isDatasetLocale, languageCookieName } from "@/i18n/config";
import { canUseDom } from "@/lib/util/can-use-dom";

const getLocaleFromCookie = () => {
  if (!canUseDom) return undefined;

  const entry = document.cookie
    .split("; ")
    .find((cookie) => cookie.startsWith(`${languageCookieName}=`));
  if (!entry) return undefined;

  return decodeURIComponent(entry.split("=").slice(1).join("="));
};

export function resolveDatasetLocale(locale?: string) {
  if (locale && isDatasetLocale(locale)) {
    return locale;
  }

  const cookieLocale = getLocaleFromCookie();
  return isDatasetLocale(cookieLocale) ? cookieLocale : defaultLocale;
}
