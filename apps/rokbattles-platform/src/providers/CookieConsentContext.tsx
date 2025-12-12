"use client";

import { createContext, type ReactNode, use, useCallback, useEffect, useState } from "react";
import { canUseDom } from "@/lib/util/canUseDom";

type CookieConsent = {
  cookieConsent?: boolean;
  country?: string;
  showConsent?: boolean;
  updateCookieConsent: (accepted: boolean) => void;
};

type CookieConsentStorage = {
  accepted: boolean;
  at: string;
  country: string;
};

const CookieConsentContext = createContext<CookieConsent>({
  cookieConsent: undefined,
  country: undefined,
  showConsent: undefined,
  updateCookieConsent: () => false,
});

const getLocalValue = (): CookieConsentStorage | null => {
  return canUseDom ? JSON.parse(window.localStorage.getItem("cookieConsent") || "null") : null;
};

const setLocalValue = (accepted: boolean, country: string) => {
  const storage: CookieConsentStorage = {
    accepted,
    at: new Date().toISOString(),
    country,
  };

  window.localStorage.setItem("cookieConsent", JSON.stringify(storage));
};

export function CookieConsentProvider({ children }: { children: ReactNode }) {
  const [showConsent, setShowConsent] = useState<boolean | undefined>();
  const [cookieConsent, setCookieConsent] = useState<boolean | undefined>();
  const [country, setCountry] = useState<string | undefined>();

  const updateCookieConsent = useCallback(
    (accepted: boolean) => {
      setCookieConsent(accepted);
      setLocalValue(accepted, country || "");
    },
    [country]
  );

  useEffect(() => {
    (async () => {
      const consent = getLocalValue();
      if (consent) {
        setCountry(consent.country);
        setCookieConsent(consent.accepted || false);
        return;
      }

      const gdpr = await fetch("/api/geolocation").then((res) => res.json());
      setCountry(gdpr.country || "");

      if (!gdpr.isGDPR) {
        setCookieConsent(false);
        updateCookieConsent(false);
      }

      setShowConsent(gdpr.isGDPR || false);
    })().catch(console.error);
  }, [updateCookieConsent]);

  return (
    <CookieConsentContext value={{ cookieConsent, country, showConsent, updateCookieConsent }}>
      {children}
    </CookieConsentContext>
  );
}

export function useCookieConsent() {
  return use(CookieConsentContext);
}
