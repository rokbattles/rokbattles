"use client";

import { useTranslations } from "next-intl";

export function ReportLoadingState() {
  const t = useTranslations("report");
  return (
    <>
      <span className="sr-only" role="status" aria-live="polite">
        {t("states.loading")}
      </span>
      <div className="space-y-6" aria-hidden="true">
        {[0, 1].map((index) => (
          <div
            key={index}
            className="h-72 animate-pulse rounded-2xl border border-zinc-950/10 bg-zinc-100/80 dark:border-white/10 dark:bg-white/5"
          />
        ))}
      </div>
    </>
  );
}
