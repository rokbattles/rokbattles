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
    <div className="fixed sm:right-4 bottom-0 sm:bottom-4 w-full sm:w-96 z-50 border-t sm:border bg-white border-zinc-200 dark:bg-zinc-800 dark:border-zinc-700 flex flex-col p-4 sm:rounded-lg">
      <Text>
        We use only essential cookies for authentication, security, and site functionality. If we
        add optional cookies in the future, you’ll be able to manage them here. Read our{" "}
        <TextLink
          href="https://rokbattles.com/legal/cookie-policy"
          target="_blank"
          rel="noopener noreferrer"
        >
          cookie policy
        </TextLink>{" "}
        for more information.
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
