"use client";

import { useTranslations } from "next-intl";
import { useEffect, useRef, useState } from "react";
import { DuelEmptyState } from "@/components/olympian-arena/duel-empty-state";
import { DuelErrorState } from "@/components/olympian-arena/duel-error-state";
import { DuelLoadingState } from "@/components/olympian-arena/duel-loading-state";
import DuelReportEntryCard from "@/components/olympian-arena/duel-report-entry-card";
import { Button } from "@/components/ui/Button";
import { Divider } from "@/components/ui/Divider";
import { Heading } from "@/components/ui/Heading";
import { useCopyToClipboard } from "@/hooks/use-copy-to-clipboard";
import { useOlympianArenaDuel } from "@/hooks/use-olympian-arena-duel";

export type DuelReportViewProps = {
  duelId: string;
};

export default function DuelReportView({ duelId }: DuelReportViewProps) {
  const t = useTranslations("duels");
  const tCommon = useTranslations("common");
  const normalizedId = duelId?.trim() ?? "";
  const parsedId = Number(normalizedId);
  const duelIdValue = Number.isFinite(parsedId) ? parsedId : null;
  const hasValidId = duelIdValue != null;

  const { data, loading, error } = useOlympianArenaDuel(duelIdValue);
  const [copiedText, copy] = useCopyToClipboard();
  const [isCopied, setIsCopied] = useState(false);
  const resetTimerRef = useRef<number>(null);

  useEffect(() => {
    return () => {
      if (resetTimerRef.current) {
        clearTimeout(resetTimerRef.current);
      }
    };
  }, []);

  const entries = data?.items ?? [];

  function handleShare() {
    if (!duelIdValue) {
      return;
    }

    copy(`https://platform.rokbattles.com/olympian-arena/${duelIdValue}`)
      .then(() => console.log("Duel report copied to clipboard", copiedText))
      .then(() => {
        setIsCopied(true);
        if (resetTimerRef.current) {
          clearTimeout(resetTimerRef.current);
        }
        resetTimerRef.current = window.setTimeout(() => setIsCopied(false), 2000);
      })
      .catch((err) => console.error("Failed to copy duel report to clipboard", err));
  }

  return (
    <section className="space-y-8">
      <div className="flex items-end justify-between gap-4">
        <Heading>{t("title")}</Heading>
        <Button className="-my-0.5" disabled={isCopied || !hasValidId} onClick={handleShare}>
          {isCopied ? tCommon("actions.copied") : tCommon("actions.share")}
        </Button>
      </div>
      <Divider />
      {!hasValidId || error ? (
        <DuelErrorState message={error ?? t("states.invalid")} />
      ) : loading ? (
        <DuelLoadingState />
      ) : entries.length === 0 ? (
        <DuelEmptyState />
      ) : (
        <div className="space-y-12">
          {entries.map((entry, index) => (
            <div key={entry.report.metadata.email_id || `${duelIdValue ?? "duel"}-${index}`}>
              <DuelReportEntryCard entry={entry} />
              {index < entries.length - 1 ? <Divider className="my-8" /> : null}
            </div>
          ))}
        </div>
      )}
    </section>
  );
}
