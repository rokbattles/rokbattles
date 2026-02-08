import "server-only";
import { cookies } from "next/headers";
import { getRequestConfig } from "next-intl/server";
import {
  DEFAULT_APP_LOCALE,
  isAppLocale,
  LOCALE_COOKIE_NAME,
} from "@/lib/i18n";

export default getRequestConfig(async (params) => {
  const store = await cookies();
  const requestedLocale = params.locale || store.get(LOCALE_COOKIE_NAME)?.value;
  const locale = isAppLocale(requestedLocale)
    ? requestedLocale
    : DEFAULT_APP_LOCALE;
  const messages = (await import(`./messages/${locale}.po`)).default;

  return {
    locale,
    messages,
  };
});
