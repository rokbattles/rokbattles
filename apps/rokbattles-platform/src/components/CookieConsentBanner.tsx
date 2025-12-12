"use client";

import { useState } from "react";
import { Button } from "@/components/ui/Button";
import { Text, TextLink } from "@/components/ui/Text";
import { useCookieConsent } from "@/providers/CookieConsentContext";

export function CookieConsentBanner() {
  const [closeBanner, setCloseBanner] = useState(false);

  const { showConsent, updateCookieConsent } = useCookieConsent();

  function handleCloseBanner() {
    setTimeout(() => setCloseBanner(true), 300);
  }

  if (!showConsent || closeBanner) {
    return null;
  }

  return (
    <div className="fixed right-4 bottom-4 w-96 z-50 border bg-white border-zinc-100 dark:bg-zinc-800 dark:border-zinc-700 flex flex-col p-4 rounded-lg">
      <Text>
        We use only strictly necessary cookies required for authentication, security, and core site
        functionality. No tracking or advertising cookies are used. Read our{" "}
        <TextLink href="https://rokbattles.com/legal/cookie-policy">cookie policy</TextLink> for
        more information.
      </Text>
      <div className="flex gap-2 mt-4">
        <Button
          color="blue"
          onClick={() => {
            updateCookieConsent(true);
            handleCloseBanner();
          }}
        >
          Got it
        </Button>
        <Button
          onClick={() => {
            updateCookieConsent(false);
            handleCloseBanner();
          }}
        >
          Dismiss
        </Button>
      </div>
    </div>
  );
}
