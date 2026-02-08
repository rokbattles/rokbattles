"use server";

import { cookies } from "next/headers";
import {
  type AppLocale,
  DEFAULT_APP_LOCALE,
  isAppLocale,
  LOCALE_COOKIE_NAME,
} from "@/lib/i18n";

export async function changeLanguage(locale: string): Promise<AppLocale> {
  const store = await cookies();
  const nextLocale = isAppLocale(locale) ? locale : DEFAULT_APP_LOCALE;

  store.set(LOCALE_COOKIE_NAME, nextLocale, {
    httpOnly: true,
    maxAge: 60 * 60 * 24 * 365,
    path: "/",
    sameSite: "lax",
    secure: process.env.NODE_ENV === "production",
  });

  return nextLocale;
}
