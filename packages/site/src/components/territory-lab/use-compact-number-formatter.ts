"use client";

import { useLocale } from "next-intl";
import { useMemo } from "react";

export function useCompactNumberFormatter(): Intl.NumberFormat {
  const locale = useLocale();
  return useMemo(
    () =>
      new Intl.NumberFormat(locale, {
        notation: "compact",
        maximumFractionDigits: 1,
      }),
    [locale]
  );
}
