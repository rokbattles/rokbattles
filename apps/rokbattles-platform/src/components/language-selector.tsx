"use client";

import { LanguageIcon } from "@heroicons/react/16/solid";
import { useRouter } from "next/navigation";
import { useTranslations } from "next-intl";
import { useCallback, useEffect, useState } from "react";
import { Button } from "@/components/ui/button";
import { Dialog, DialogActions, DialogBody, DialogTitle } from "@/components/ui/dialog";
import { Description, Fieldset, Label, Legend } from "@/components/ui/fieldset";
import { Radio, RadioField, RadioGroup } from "@/components/ui/radio";
import { SidebarItem, SidebarLabel } from "@/components/ui/sidebar";
import {
  datasetLanguageOptions,
  defaultLocale,
  isDatasetLocale,
  isSiteLocale,
  languageCookieName,
  siteLanguageOptions,
} from "@/i18n/config";

const COOKIE_MAX_AGE_SECONDS = 60 * 60 * 24 * 30;

const datasetOnlyLanguageOptions = datasetLanguageOptions.filter(
  (option) => !isSiteLocale(option.locale)
);

const getLocaleFromCookie = () => {
  if (typeof document === "undefined") return defaultLocale;

  const entry = document.cookie
    .split("; ")
    .find((cookie) => cookie.startsWith(`${languageCookieName}=`));
  if (!entry) return defaultLocale;

  const value = decodeURIComponent(entry.split("=").slice(1).join("="));
  return isDatasetLocale(value) ? value : defaultLocale;
};

const setLocaleCookie = (locale: string) => {
  if (typeof document === "undefined") return;

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
    datasetLanguageOptions.find((option) => option.locale === currentLocale) ??
    datasetLanguageOptions[0];

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
    const nextLocale = isDatasetLocale(selectedLocale) ? selectedLocale : defaultLocale;
    setLocaleCookie(nextLocale);
    setCurrentLocale(nextLocale);
    setIsOpen(false);
    router.refresh();
  }, [router, selectedLocale]);

  return (
    <>
      <SidebarItem onClick={handleOpen} aria-label={t("select")}>
        <LanguageIcon />
        <SidebarLabel>{currentLanguage.label}</SidebarLabel>
      </SidebarItem>
      <Dialog open={isOpen} onClose={handleClose} size="xl">
        <DialogTitle>{t("title")}</DialogTitle>
        <DialogBody>
          <RadioGroup
            value={selectedLocale}
            onChange={setSelectedLocale}
            aria-label={t("select")}
            name="platformLanguage"
          >
            <div className="space-y-8">
              <Fieldset>
                <Legend>{t("site.label")}</Legend>
                <Description>{t("site.description")}</Description>
                <div data-slot="control" className="grid gap-3 sm:grid-cols-2">
                  {siteLanguageOptions.map((option) => (
                    <RadioField key={option.locale}>
                      <Radio value={option.locale} />
                      <Label className="truncate">{option.label}</Label>
                    </RadioField>
                  ))}
                </div>
              </Fieldset>
              <Fieldset>
                <Legend>{t("dataset.label")}</Legend>
                <Description>{t("dataset.description")}</Description>
                <div data-slot="control" className="grid gap-3 sm:grid-cols-2">
                  {datasetOnlyLanguageOptions.map((option) => (
                    <RadioField key={option.locale}>
                      <Radio value={option.locale} />
                      <Label className="truncate">{option.label}</Label>
                    </RadioField>
                  ))}
                </div>
              </Fieldset>
            </div>
          </RadioGroup>
        </DialogBody>
        <DialogActions>
          <Button outline onClick={() => handleClose(false)}>
            {tCommon("actions.cancel")}
          </Button>
          <Button onClick={handleSave}>{t("save")}</Button>
        </DialogActions>
      </Dialog>
    </>
  );
}
