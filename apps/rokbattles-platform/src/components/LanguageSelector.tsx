"use client";

import { useRouter } from "next/navigation";
import { useCallback, useEffect, useState } from "react";
import Flag from "react-flagpack";
import { Button } from "@/components/ui/Button";
import { Dialog, DialogActions, DialogBody, DialogTitle } from "@/components/ui/Dialog";
import { Label } from "@/components/ui/Fieldset";
import { Radio, RadioField, RadioGroup } from "@/components/ui/Radio";
import { SidebarItem, SidebarLabel } from "@/components/ui/Sidebar";
import {
  defaultLocale,
  isSupportedLocale,
  languageCookieName,
  languageOptions,
} from "@/i18n/config";

const COOKIE_MAX_AGE_SECONDS = 60 * 60 * 24 * 30;

const getLocaleFromCookie = () => {
  if (typeof document === "undefined") return defaultLocale;

  const entry = document.cookie
    .split("; ")
    .find((cookie) => cookie.startsWith(`${languageCookieName}=`));
  if (!entry) return defaultLocale;

  const value = decodeURIComponent(entry.split("=").slice(1).join("="));
  return isSupportedLocale(value) ? value : defaultLocale;
};

const setLocaleCookie = (locale: string) => {
  if (typeof document === "undefined") return;

  // biome-ignore lint/suspicious/noDocumentCookie: ignore
  document.cookie = `${languageCookieName}=${encodeURIComponent(
    locale
  )}; Max-Age=${COOKIE_MAX_AGE_SECONDS}; Path=/; SameSite=Lax; Secure`;
};

export function LanguageSelector() {
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
    const nextLocale = isSupportedLocale(selectedLocale) ? selectedLocale : defaultLocale;
    setLocaleCookie(nextLocale);
    setCurrentLocale(nextLocale);
    setIsOpen(false);
    router.refresh();
  }, [router, selectedLocale]);

  return (
    <>
      <SidebarItem onClick={handleOpen} aria-label="Select language">
        <span data-slot="icon" className="flex items-center justify-center">
          <Flag code={currentLanguage.flagCode} size="m" hasBorder={false} />
        </span>
        <SidebarLabel>{currentLanguage.label}</SidebarLabel>
      </SidebarItem>
      <Dialog open={isOpen} onClose={handleClose} size="xl">
        <DialogTitle>Language</DialogTitle>
        <DialogBody>
          <RadioGroup
            value={selectedLocale}
            onChange={setSelectedLocale}
            aria-label="Select language"
            name="platformLanguage"
          >
            {languageOptions.map((option) => (
              <RadioField key={option.locale}>
                <Radio value={option.locale} />
                <Label className="flex items-center gap-3">
                  <Flag code={option.flagCode} size="m" hasBorder={false} className="shrink-0" />
                  <span className="truncate">{option.label}</span>
                </Label>
              </RadioField>
            ))}
          </RadioGroup>
        </DialogBody>
        <DialogActions>
          <Button outline onClick={() => handleClose(false)}>
            Cancel
          </Button>
          <Button onClick={handleSave}>Save</Button>
        </DialogActions>
      </Dialog>
    </>
  );
}
