"use server";

import { cookies } from "next/headers";
import type { Locale } from "next-intl";

export async function changeLanguage(locale: Locale) {
  const store = await cookies();
  store.set("ROKB_LOCALE", locale);
}
