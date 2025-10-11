"use client";

import { useMemo } from "react";
import { ReportEntryCard } from "@/components/report/ReportEntryCard";
import { ReportTimelineChart } from "@/components/report/ReportTimelineChart";
import { Button } from "@/components/ui/Button";
import { Divider } from "@/components/ui/Divider";
import { Heading, Subheading } from "@/components/ui/Heading";
import { Text } from "@/components/ui/Text";
import { useCopyToClipboard } from "@/hooks/useCopyToClipboard";
import { useReport } from "@/hooks/useReport";
import type { ReportEntry } from "@/lib/types/report";

type ReportViewProps = {
  hash: string;
};

export function ReportView({ hash }: ReportViewProps) {
  const normalizedHash = hash?.trim() ?? "";

  const { data, loading, error } = useReport(normalizedHash.length > 0 ? normalizedHash : null);
  const [copiedText, copy] = useCopyToClipboard();

  const entries: ReportEntry[] = useMemo(() => data?.items ?? [], [data?.items]);
  const summary = data?.battleResults;

  function handleShare() {
    copy(`https://rokbattles.com/report/${normalizedHash}`)
      .then(() => console.log("Battle report copied to clipboard", copiedText))
      .catch((err) => console.error("Failed to copy battle report to clipboard", err));
  }

  return (
    <section className="space-y-8">
      <div className="flex items-end justify-between gap-4">
        <Heading>Report</Heading>
        <Button className="-my-0.5" onClick={handleShare}>
          Share
        </Button>
      </div>
      <Divider />
      {loading ? (
        <ReportLoadingState />
      ) : error ? (
        <ReportErrorState message={error} />
      ) : entries.length === 0 ? (
        <ReportEmptyState />
      ) : (
        <div className="space-y-12">
          {summary && summary.timeline.length > 1 ? (
            <>
              <div className="space-y-4">
                <Subheading>Battle timeline</Subheading>
                <ReportTimelineChart summary={summary} />
              </div>
              <Divider />
            </>
          ) : null}

          <div className="space-y-12">
            {entries.map((entry, index) => (
              <div key={entry.hash}>
                <ReportEntryCard entry={entry} />
                {index < entries.length - 1 ? <Divider className="my-8" /> : null}
              </div>
            ))}
          </div>
        </div>
      )}
    </section>
  );
}

function ReportLoadingState() {
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

function ReportErrorState({ message }: { message: string }) {
  return <Text>We could not load this report. {message}</Text>;
}

function ReportEmptyState() {
  return <Text>No battles were found for this hash.</Text>;
}
