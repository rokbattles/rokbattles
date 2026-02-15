export const languageCookieName = "platformLanguage";

export const defaultLocale = "en";

export const siteSupportedLocales = ["en", "es", "ko"] as const;
export type SiteLocale = (typeof siteSupportedLocales)[number];

export const datasetSupportedLocales = [
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
export type DatasetLocale = (typeof datasetSupportedLocales)[number];

const localeMeta: Record<DatasetLocale, string> = {
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

export const siteLanguageOptions = [...siteSupportedLocales]
  .map((locale) => ({
    locale,
    label: localeMeta[locale],
  }))
  .sort(sortByLanguageLabel);

export const datasetLanguageOptions = [...datasetSupportedLocales]
  .map((locale) => ({
    locale,
    label: localeMeta[locale],
  }))
  .sort(sortByLanguageLabel);

export const isSiteLocale = (value?: string): value is SiteLocale =>
  Boolean(value) && siteSupportedLocales.includes(value as SiteLocale);

export const isDatasetLocale = (value?: string): value is DatasetLocale =>
  Boolean(value) && datasetSupportedLocales.includes(value as DatasetLocale);
