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

const localeMeta: Record<DatasetLocale, { label: string }> = {
  en: { label: "English" },
  fr: { label: "Français" },
  de: { label: "Deutsch" },
  ru: { label: "Русский" },
  pt: { label: "Português" },
  es: { label: "Español" },
  it: { label: "Italiano" },
  zh_CN: { label: "简体中文" },
  zh_TW: { label: "繁體中文" },
  ko: { label: "한국어" },
  id: { label: "Indonesia" },
  tr: { label: "Türkçe" },
  th: { label: "ไทย" },
  ms: { label: "Melayu" },
  vi: { label: "Tiếng Việt" },
  ar: { label: "العربية" },
  ja: { label: "日本語" },
  pl: { label: "Polski" },
};

const sortByLanguageLabel = <T extends { label: string }>(a: T, b: T) =>
  a.label.localeCompare(b.label);

export const siteLanguageOptions = [...siteSupportedLocales]
  .map((locale) => ({
    locale,
    ...localeMeta[locale],
  }))
  .sort(sortByLanguageLabel);

export const datasetLanguageOptions = [...datasetSupportedLocales]
  .map((locale) => ({
    locale,
    ...localeMeta[locale],
  }))
  .sort(sortByLanguageLabel);

export const isSiteLocale = (value?: string): value is SiteLocale =>
  Boolean(value) && siteSupportedLocales.includes(value as SiteLocale);

export const isDatasetLocale = (value?: string): value is DatasetLocale =>
  Boolean(value) && datasetSupportedLocales.includes(value as DatasetLocale);
