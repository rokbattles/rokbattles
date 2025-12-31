export const languageCookieName = "platformLanguage";

export const defaultLocale = "en";

// temporarily disabling German (de)
export const supportedLocales = ["en", "es", "ko"] as const;

export type SupportedLocale = (typeof supportedLocales)[number];

export const languageOptions: ReadonlyArray<{
  locale: SupportedLocale;
  label: string;
  flagCode: string;
}> = [
  { locale: "en", label: "English", flagCode: "US" },
  { locale: "es", label: "Spanish", flagCode: "ES" },
  // { locale: "de", label: "German", flagCode: "DE" },
  { locale: "ko", label: "Korean", flagCode: "KR" },
];

export const isSupportedLocale = (value?: string): value is SupportedLocale =>
  Boolean(value) && supportedLocales.includes(value as SupportedLocale);
