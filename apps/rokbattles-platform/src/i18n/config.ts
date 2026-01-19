export const languageCookieName = "platformLanguage";

export const defaultLocale = "en";

export const siteSupportedLocales = ["en", "es", "ko"] as const;
export type SiteLocale = (typeof siteSupportedLocales)[number];

export const datasetSupportedLocales = ["de", "en", "es", "fr", "ko", "pl", "zh_CN"] as const;
export type DatasetLocale = (typeof datasetSupportedLocales)[number];

const localeMeta: Record<DatasetLocale, { label: string; flagCode: string }> = {
  de: { label: "German", flagCode: "DE" },
  en: { label: "English", flagCode: "US" },
  es: { label: "Spanish", flagCode: "ES" },
  fr: { label: "French", flagCode: "FR" },
  ko: { label: "Korean", flagCode: "KR" },
  pl: { label: "Polish", flagCode: "PL" },
  zh_CN: { label: "Chinese (Simplified)", flagCode: "CN" },
};

export const siteLanguageOptions = siteSupportedLocales.map((locale) => ({
  locale,
  ...localeMeta[locale],
}));

export const datasetLanguageOptions = datasetSupportedLocales.map((locale) => ({
  locale,
  ...localeMeta[locale],
}));

export const isSiteLocale = (value?: string): value is SiteLocale =>
  Boolean(value) && siteSupportedLocales.includes(value as SiteLocale);

export const isDatasetLocale = (value?: string): value is DatasetLocale =>
  Boolean(value) && datasetSupportedLocales.includes(value as DatasetLocale);
