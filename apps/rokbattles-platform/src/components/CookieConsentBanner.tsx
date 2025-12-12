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
        We use only essential cookies for authentication, security, and site functionality. If we
        add optional cookies in the future, you’ll be able to manage them here. Read our{" "}
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
          Accept
        </Button>
        <Button
          onClick={() => {
            updateCookieConsent(false);
            handleCloseBanner();
          }}
        >
          Reject
        </Button>
      </div>
    </div>
  );
}
