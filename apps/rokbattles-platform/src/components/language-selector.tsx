"use client";

import { useRouter } from "next/navigation";
import { useTranslations } from "next-intl";
import { useCallback, useEffect, useState } from "react";
import Flag from "react-flagpack";
import { Button } from "@/components/ui/button";
import {
  Dialog,
  DialogActions,
  DialogBody,
  DialogTitle,
} from "@/components/ui/dialog";
import { Label } from "@/components/ui/fieldset";
import { Radio, RadioField, RadioGroup } from "@/components/ui/radio";
import { SidebarItem, SidebarLabel } from "@/components/ui/sidebar";
import {
  defaultLocale,
  isSupportedLocale,
  languageCookieName,
  languageOptions,
} from "@/i18n/config";

const COOKIE_MAX_AGE_SECONDS = 60 * 60 * 24 * 30;

const getLocaleFromCookie = () => {
  if (typeof document === "undefined") {
    return defaultLocale;
  }

  const entry = document.cookie
    .split("; ")
    .find((cookie) => cookie.startsWith(`${languageCookieName}=`));
  if (!entry) {
    return defaultLocale;
  }

  const value = decodeURIComponent(entry.split("=").slice(1).join("="));
  return isSupportedLocale(value) ? value : defaultLocale;
};

const setLocaleCookie = (locale: string) => {
  if (typeof document === "undefined") {
    return;
  }

  // biome-ignore lint/suspicious/noDocumentCookie: ignore
  document.cookie = `${languageCookieName}=${encodeURIComponent(
    locale
  )}; Max-Age=${COOKIE_MAX_AGE_SECONDS}; Path=/; SameSite=Lax; Secure`;
};

export function LanguageSelector() {
  const t = useTranslations("language");
  const tCommon = useTranslations("common");
  const router = useRouter();
  const [isOpen, setIsOpen] = useState(false);
  const [currentLocale, setCurrentLocale] = useState(defaultLocale);
  const [selectedLocale, setSelectedLocale] = useState(defaultLocale);

  useEffect(() => {
    const locale = getLocaleFromCookie();
    setCurrentLocale(locale);
    setSelectedLocale(locale);
  }, []);

  const currentLanguage =
    languageOptions.find((option) => option.locale === currentLocale) ??
    languageOptions[0];

  const handleOpen = useCallback(() => {
    setSelectedLocale(currentLocale);
    setIsOpen(true);
  }, [currentLocale]);

  const handleClose = useCallback(
    (open: boolean) => {
      setIsOpen(open);
      if (!open) {
        setSelectedLocale(currentLocale);
      }
    },
    [currentLocale]
  );

  const handleSave = useCallback(() => {
    const nextLocale = isSupportedLocale(selectedLocale)
      ? selectedLocale
      : defaultLocale;
    setLocaleCookie(nextLocale);
    setCurrentLocale(nextLocale);
    setIsOpen(false);
    router.refresh();
  }, [router, selectedLocale]);

  return (
    <>
      <SidebarItem aria-label={t("select")} onClick={handleOpen}>
        <span className="flex items-center justify-center" data-slot="icon">
          <Flag code={currentLanguage.flagCode} hasBorder={false} size="m" />
        </span>
        <SidebarLabel>{currentLanguage.label}</SidebarLabel>
      </SidebarItem>
      <Dialog onClose={handleClose} open={isOpen} size="xl">
        <DialogTitle>{t("title")}</DialogTitle>
        <DialogBody>
          <RadioGroup
            aria-label={t("select")}
            name="platformLanguage"
            onChange={setSelectedLocale}
            value={selectedLocale}
          >
            {languageOptions.map((option) => (
              <RadioField key={option.locale}>
                <Radio value={option.locale} />
                <Label className="flex items-center gap-3">
                  <Flag
                    className="shrink-0"
                    code={option.flagCode}
                    hasBorder={false}
                    size="m"
                  />
                  <span className="truncate">{option.label}</span>
                </Label>
              </RadioField>
            ))}
          </RadioGroup>
        </DialogBody>
        <DialogActions>
          <Button onClick={() => handleClose(false)} outline>
            {tCommon("actions.cancel")}
          </Button>
          <Button onClick={handleSave}>{t("save")}</Button>
        </DialogActions>
      </Dialog>
    </>
  );
}
