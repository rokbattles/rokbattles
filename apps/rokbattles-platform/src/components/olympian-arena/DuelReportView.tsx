"use client";

import { useEffect, useMemo, useRef, useState } from "react";
import DuelReportEntryCard, {
  type DuelReportPayload,
} from "@/components/olympian-arena/DuelReportEntryCard";
import { Button } from "@/components/ui/Button";
import { Divider } from "@/components/ui/Divider";
import { Heading } from "@/components/ui/Heading";
import { Text } from "@/components/ui/Text";
import { useCopyToClipboard } from "@/hooks/useCopyToClipboard";
import { useOlympianArenaDuel } from "@/hooks/useOlympianArenaDuel";

export type DuelReportViewProps = {
  duelId: string;
};

export default function DuelReportView({ duelId }: DuelReportViewProps) {
  const normalizedId = duelId?.trim() ?? "";
  const parsedId = Number(normalizedId);
  const duelIdValue = Number.isFinite(parsedId) && parsedId > 0 ? Math.trunc(parsedId) : null;
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

  const entries = useMemo(() => data?.items ?? [], [data?.items]);

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
        <Heading>Duel Report</Heading>
        <Button className="-my-0.5" disabled={isCopied || !hasValidId} onClick={handleShare}>
          {isCopied ? "Copied" : "Share"}
        </Button>
      </div>
      <Divider />
      {!hasValidId || error ? (
        <DuelErrorState message={error ?? "We could not load this duel."} />
      ) : loading ? (
        <DuelLoadingState />
      ) : entries.length === 0 ? (
        <DuelEmptyState />
      ) : (
        <div className="space-y-12">
          {entries.map((entry, index) => (
            <div
              key={
                (entry.report as DuelReportPayload)?.metadata?.email_id ??
                `${duelIdValue ?? "duel"}-${index}`
              }
            >
              <DuelReportEntryCard entry={entry} />
              {index < entries.length - 1 ? <Divider className="my-8" /> : null}
            </div>
          ))}
        </div>
      )}
    </section>
  );
}

function DuelLoadingState() {
  return (
    <div className="space-y-6">
      {[0, 1].map((index) => (
        <div
          key={index}
          className="h-72 animate-pulse rounded-2xl border border-zinc-950/10 bg-zinc-100/80 dark:border-white/10 dark:bg-white/5"
        />
      ))}
    </div>
  );
}

function DuelErrorState({ message }: { message: string }) {
  return <Text>We could not load this duel. {message}</Text>;
}

function DuelEmptyState() {
  return <Text>No reports were found for this duel.</Text>;
}
