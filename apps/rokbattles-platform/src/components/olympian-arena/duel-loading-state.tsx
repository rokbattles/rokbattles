"use client";

import { useTranslations } from "next-intl";

export function DuelLoadingState() {
  const t = useTranslations("duels");
  return (
    <>
      <span aria-live="polite" className="sr-only" role="status">
        {t("states.loading")}
      </span>
      <div aria-hidden="true" className="space-y-6">
        {[0, 1].map((index) => (
          <div
            className="h-72 animate-pulse rounded-2xl border border-zinc-950/10 bg-zinc-100/80 dark:border-white/10 dark:bg-white/5"
            key={index}
          />
        ))}
      </div>
    </>
  );
}
