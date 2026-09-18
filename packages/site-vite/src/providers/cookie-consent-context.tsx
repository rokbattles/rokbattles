import { createContext, type ReactNode, useCallback, useState } from "react";

export type OptionalCookiePreferences = {
  functional: boolean;
  analytics: boolean;
  marketing: boolean;
};

type CookieConsent = {
  at: string;
  categories: OptionalCookiePreferences & { necessary: true };
};

export const defaultCookiePreferences: OptionalCookiePreferences = {
  functional: false,
  analytics: false,
  marketing: false,
};

type CookieConsentContextValue = {
  consent: CookieConsent | null;
  isOpen: boolean;
  open: () => void;
  close: () => void;
  update: (preferences: OptionalCookiePreferences) => void;
};

export const CookieConsentContext = createContext({} as CookieConsentContextValue);

export function CookieConsentProvider({ children }: { children: ReactNode }) {
  const [consent, setConsent] = useState<CookieConsent | null>(() => {
    try {
      return JSON.parse(localStorage.getItem("__rokb_cc") || "null");
    } catch {
      return null;
    }
  });
  const [isOpen, setIsOpen] = useState(false);
  const open = useCallback(() => setIsOpen(true), []);
  const close = useCallback(() => setIsOpen(false), []);
  const update = useCallback((preferences: OptionalCookiePreferences) => {
    const next: CookieConsent = {
      at: new Date().toISOString(),
      categories: {
        ...preferences,
        necessary: true,
      },
    };
    try {
      localStorage.setItem("__rokb_cc", JSON.stringify(next));
    } catch {}
    setConsent(next);
    setIsOpen(false);
  }, []);

  return (
    <CookieConsentContext value={{ consent, isOpen, open, close, update }}>
      {children}
    </CookieConsentContext>
  );
}
