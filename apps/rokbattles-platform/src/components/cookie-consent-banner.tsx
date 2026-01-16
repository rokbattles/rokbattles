"use client";

import { useTranslations } from "next-intl";
import { useState } from "react";
import { Button } from "@/components/ui/button";
import { Text, TextLink } from "@/components/ui/text";
import { useCookieConsent } from "@/providers/cookie-consent-context";

export function CookieConsentBanner() {
  const [closeBanner, setCloseBanner] = useState(false);
  const t = useTranslations("cookieConsent");

  const { showConsent, updateCookieConsent } = useCookieConsent();

  function handleCloseBanner() {
    setTimeout(() => setCloseBanner(true), 300);
  }

  if (!showConsent || closeBanner) {
    return null;
  }

  return (
    <div className="fixed bottom-0 z-50 flex w-full flex-col border-zinc-200 border-t bg-white p-4 sm:right-4 sm:bottom-4 sm:w-96 sm:rounded-lg sm:border dark:border-zinc-700 dark:bg-zinc-800">
      <Text>
        {t.rich("message", {
          link: (chunks) => (
            <TextLink href="/legal/cookie-policy">{chunks}</TextLink>
          ),
        })}
      </Text>
      <div className="mt-4 flex gap-2">
        <Button
          color="blue"
          onClick={() => {
            updateCookieConsent(true);
            handleCloseBanner();
          }}
        >
          {t("accept")}
        </Button>
        <Button
          onClick={() => {
            updateCookieConsent(false);
            handleCloseBanner();
          }}
        >
          {t("reject")}
        </Button>
      </div>
    </div>
  );
}
