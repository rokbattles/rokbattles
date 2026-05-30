import { cookies } from "next/headers";
import { getRequestConfig } from "next-intl/server";
import { defaultLocale, isLocale, languageCookieName } from "./config";
import { loadCatalogMessages, mergeMessagesWithFallback, toIntlLocale } from "./message-catalog";

export default getRequestConfig(async () => {
  const cookieStore = await cookies();
  const cookieLocale = cookieStore.get(languageCookieName)?.value;

  const locale = isLocale(cookieLocale) ? cookieLocale : defaultLocale;
  const fallbackMessages = await loadCatalogMessages(defaultLocale);

  if (!fallbackMessages) {
    throw new Error(`Missing required default locale catalog: ${defaultLocale}`);
  }

  if (locale === defaultLocale) {
    return {
      locale: toIntlLocale(locale),
      messages: fallbackMessages,
    };
  }

  const localeMessages = await loadCatalogMessages(locale);

  return {
    locale: toIntlLocale(locale),
    messages: localeMessages
      ? mergeMessagesWithFallback(localeMessages, fallbackMessages)
      : fallbackMessages,
  };
});
