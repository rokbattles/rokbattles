"use client";

import { LanguageIcon } from "@heroicons/react/16/solid";
import { useRouter } from "next/navigation";
import { useExtracted } from "next-intl";
import { useCallback, useEffect, useState } from "react";
import { Button } from "@/components/ui/button";
import { Dialog, DialogActions, DialogBody, DialogTitle } from "@/components/ui/dialog";
import { Description, Fieldset, Label } from "@/components/ui/fieldset";
import { Radio, RadioField, RadioGroup } from "@/components/ui/radio";
import { SidebarItem, SidebarLabel } from "@/components/ui/sidebar";
import { defaultLocale, isLocale, languageCookieName, languageOptions } from "@/i18n/config";

const COOKIE_MAX_AGE_SECONDS = 60 * 60 * 24 * 30;

const getLocaleFromCookie = () => {
  if (typeof document === "undefined") return defaultLocale;

  const entry = document.cookie
    .split("; ")
    .find((cookie) => cookie.startsWith(`${languageCookieName}=`));
  if (!entry) return defaultLocale;

  const value = decodeURIComponent(entry.split("=").slice(1).join("="));
  return isLocale(value) ? value : defaultLocale;
};

const setLocaleCookie = (locale: string) => {
  if (typeof document === "undefined") return;

  // biome-ignore lint/suspicious/noDocumentCookie: ignore
  document.cookie = `${languageCookieName}=${encodeURIComponent(
    locale
  )}; Max-Age=${COOKIE_MAX_AGE_SECONDS}; Path=/; SameSite=Lax; Secure`;
};

export function LanguageSelector() {
  const t = useExtracted();
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
    languageOptions.find((option) => option.locale === currentLocale) ?? languageOptions[0];

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
    const nextLocale = isLocale(selectedLocale) ? selectedLocale : defaultLocale;
    setLocaleCookie(nextLocale);
    setCurrentLocale(nextLocale);
    setIsOpen(false);
    router.refresh();
  }, [router, selectedLocale]);

  return (
    <>
      <SidebarItem onClick={handleOpen} aria-label={t("Select language")}>
        <LanguageIcon />
        <SidebarLabel>{currentLanguage.label}</SidebarLabel>
      </SidebarItem>
      <Dialog open={isOpen} onClose={handleClose} size="xl">
        <DialogTitle>{t("Language")}</DialogTitle>
        <DialogBody>
          <RadioGroup
            value={selectedLocale}
            onChange={setSelectedLocale}
            aria-label={t("Select language")}
            name={languageCookieName}
          >
            <Fieldset>
              <Description>
                {t(
                  "Languages besides English may be incomplete. Untranslated text falls back to English."
                )}
              </Description>
              <div data-slot="control" className="grid gap-3 sm:grid-cols-2">
                {languageOptions.map((option) => (
                  <RadioField key={option.locale}>
                    <Radio value={option.locale} />
                    <Label className="truncate">{option.label}</Label>
                  </RadioField>
                ))}
              </div>
            </Fieldset>
          </RadioGroup>
        </DialogBody>
        <DialogActions>
          <Button outline onClick={() => handleClose(false)}>
            {t("Cancel")}
          </Button>
          <Button onClick={handleSave}>{t("Save")}</Button>
        </DialogActions>
      </Dialog>
    </>
  );
}
