"use client";

import { useLocale } from "next-intl";
import {
  createContext,
  type ReactNode,
  useCallback,
  useEffect,
  useMemo,
  useRef,
  useState,
} from "react";
import { resolveLocale } from "@/i18n/locale";

type GameTranslationResponse = Record<string, string | null>;

type GameTranslationsContextValue = {
  registerKey: (translationKey: string) => () => void;
  translations: GameTranslationResponse;
};

export const GameTranslationsContext = createContext<GameTranslationsContextValue | null>(null);

function isGameTranslationResponse(value: unknown): value is GameTranslationResponse {
  if (!value || typeof value !== "object" || Array.isArray(value)) return false;

  return Object.values(value).every((translation) => {
    return translation === null || typeof translation === "string";
  });
}

export function GameTranslationsProvider({ children }: { children: ReactNode }) {
  const locale = resolveLocale(useLocale());
  const currentLocaleRef = useRef(locale);

  const keyUsageCountsRef = useRef(new Map<string, number>());
  const requestedKeysRef = useRef(new Set<string>());
  const previousLocaleRef = useRef(locale);
  const [registeredKeys, setRegisteredKeys] = useState<ReadonlySet<string>>(() => new Set());
  const [translations, setTranslations] = useState<GameTranslationResponse>({});

  useEffect(() => {
    currentLocaleRef.current = locale;
  }, [locale]);

  const registerKey = useCallback((translationKey: string) => {
    const usageCount = keyUsageCountsRef.current.get(translationKey) ?? 0;
    keyUsageCountsRef.current.set(translationKey, usageCount + 1);

    if (usageCount === 0) {
      setRegisteredKeys((currentKeys) => new Set(currentKeys).add(translationKey));
    }

    return () => {
      const currentUsageCount = keyUsageCountsRef.current.get(translationKey) ?? 0;
      if (currentUsageCount > 1) {
        keyUsageCountsRef.current.set(translationKey, currentUsageCount - 1);
        return;
      }

      keyUsageCountsRef.current.delete(translationKey);
      setRegisteredKeys((currentKeys) => {
        const nextKeys = new Set(currentKeys);
        nextKeys.delete(translationKey);
        return nextKeys;
      });
    };
  }, []);

  useEffect(() => {
    if (previousLocaleRef.current === locale) return;

    previousLocaleRef.current = locale;
    requestedKeysRef.current.clear();
    setTranslations({});
  }, [locale]);

  useEffect(() => {
    const keysToRequest = [...registeredKeys]
      .filter((translationKey) => !requestedKeysRef.current.has(translationKey))
      .sort();
    if (keysToRequest.length === 0) return;

    const batchTimer = window.setTimeout(async () => {
      for (const translationKey of keysToRequest) {
        requestedKeysRef.current.add(translationKey);
      }

      const requestLocale = locale;

      try {
        const response = await fetch("/proxy/v1/game/translate", {
          method: "POST",
          headers: { "content-type": "application/json" },
          body: JSON.stringify({
            key: keysToRequest.join(","),
            lang: requestLocale,
          }),
        });

        if (!response.ok) {
          throw new Error(`Game translation request failed with status ${response.status}`);
        }

        const result: unknown = await response.json();
        if (!isGameTranslationResponse(result)) {
          throw new Error("Game translation response has an unexpected shape");
        }

        if (currentLocaleRef.current !== requestLocale) return;

        setTranslations((currentTranslations) => ({ ...currentTranslations, ...result }));
      } catch (requestError) {
        if (currentLocaleRef.current !== requestLocale) return;

        for (const translationKey of keysToRequest) {
          requestedKeysRef.current.delete(translationKey);
        }
        console.error("Game translation request failed", requestError);
      }
    }, 0);

    return () => window.clearTimeout(batchTimer);
  }, [locale, registeredKeys]);

  const contextValue = useMemo<GameTranslationsContextValue>(
    () => ({ registerKey, translations }),
    [registerKey, translations]
  );

  return (
    <GameTranslationsContext.Provider value={contextValue}>
      {children}
    </GameTranslationsContext.Provider>
  );
}
