import { cookies } from "next/headers";
import { getRequestConfig } from "next-intl/server";

export default getRequestConfig(async () => {
  const cookieStore = await cookies();
  const cookieLocale = cookieStore.get("platformLanguage")?.value;

  const locale = cookieLocale || "en";

  return {
    locale,
    messages: (await import(`./messages/${locale}.json`)).default,
  };
});
