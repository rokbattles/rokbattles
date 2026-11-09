"use client";

import { useEffect, useState } from "react";
import { Subheading } from "@/components/ui/heading";
import { fetchLootExplorerItems, type StrangeIncidentsLootDocument } from "@/lib/loot-explorer/api";
import { LootExplorerLayout } from "./loot-explorer-layout";
import { LootExplorerStatus } from "./loot-explorer-status";
import { LootExplorerSummary } from "./loot-explorer-summary";
import { LootTable } from "./loot-table";

const numberFormatter = new Intl.NumberFormat("en-US");

export function StrangeIncidentsExplorer() {
  const [items, setItems] = useState<StrangeIncidentsLootDocument[]>([]);
  const [status, setStatus] = useState<"loading" | "ready" | "error">("loading");

  useEffect(() => {
    let ignore = false;

    fetchLootExplorerItems<StrangeIncidentsLootDocument>("strange-incidents")
      .then((response) => {
        if (!ignore) {
          setItems(response.items);
          setStatus("ready");
        }
      })
      .catch(() => {
        if (!ignore) setStatus("error");
      });

    return () => {
      ignore = true;
    };
  }, []);

  if (status === "loading") {
    return (
      <LootExplorerStatus active="strange-incidents" message="Loading loot explorer data..." />
    );
  }

  if (status === "error") {
    return (
      <LootExplorerStatus active="strange-incidents" message="Failed to load loot explorer data." />
    );
  }

  return <StrangeIncidentsExplorerContent items={items} />;
}

export function StrangeIncidentsExplorerContent({
  items,
}: {
  items: StrangeIncidentsLootDocument[];
}) {
  const item = items.find((candidate) => candidate.kind === 70_005);

  return (
    <LootExplorerLayout active="strange-incidents">
      <LootExplorerSummary
        generatedAt={item?.refreshedAt}
        items={[{ label: "Results", value: item?.totals.results ?? 0 }]}
      />
      {item && item.totals.unmatchedResults > 0 ? (
        <p className="rounded-lg px-4 py-3 text-sm/6 bg-amber-500/10 text-amber-200">
          {`Rates exclude ${
            Math.abs(item.totals.unmatchedResults) === 1
              ? `${numberFormatter.format(item.totals.unmatchedResults)} reward result`
              : `${numberFormatter.format(item.totals.unmatchedResults)} reward results`
          } that could not be assigned to the six observed reward slots.`}
        </p>
      ) : null}
      <div className="space-y-8">
        {item?.slots.map((slot) => (
          <section key={slot.slot} className="space-y-3">
            <Subheading level={3}>{`Slot ${slot.slot.toString()}`}</Subheading>
            <LootTable
              loot={slot.loot}
              noItem={
                slot.noItemResults > 0
                  ? { dropRate: slot.noItemRate, results: slot.noItemResults }
                  : undefined
              }
            />
          </section>
        ))}
      </div>
    </LootExplorerLayout>
  );
}
