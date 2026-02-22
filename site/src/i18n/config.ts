export const languageCookieName = "platformLanguage";

export const defaultLocale = "en";

export const supportedLocales = [
  "en",
  "fr",
  "de",
  "ru",
  "pt",
  "es",
  "it",
  "zh_CN",
  "zh_TW",
  "ko",
  "id",
  "tr",
  "th",
  "ms",
  "vi",
  "ar",
  "ja",
  "pl",
] as const;
export type Locale = (typeof supportedLocales)[number];

const localeMeta: Record<Locale, string> = {
  en: "English",
  fr: "Français",
  de: "Deutsch",
  ru: "Русский",
  pt: "Português",
  es: "Español",
  it: "Italiano",
  zh_CN: "简体中文",
  zh_TW: "繁體中文",
  ko: "한국어",
  id: "Indonesia",
  tr: "Türkçe",
  th: "ไทย",
  ms: "Melayu",
  vi: "Tiếng Việt",
  ar: "العربية",
  ja: "日本語",
  pl: "Polski",
};

const sortByLanguageLabel = <T extends { label: string }>(a: T, b: T) =>
  a.label.localeCompare(b.label);

export const languageOptions = [...supportedLocales]
  .map((locale) => ({
    locale,
    label: localeMeta[locale],
  }))
  .sort(sortByLanguageLabel);

export const isLocale = (value?: string): value is Locale =>
  Boolean(value) && supportedLocales.includes(value as Locale);
