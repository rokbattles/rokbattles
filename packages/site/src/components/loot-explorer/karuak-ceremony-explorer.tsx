"use client";

import { useEffect, useState } from "react";
import { Subheading } from "@/components/ui/heading";
import { fetchLootExplorerItems, type KaruakCeremonyLootDocument } from "@/lib/loot-explorer/api";
import { findKaruakBoss, karuakBosses } from "@/lib/loot-explorer/catalog";
import { LootExplorerFilters } from "./loot-explorer-filters";
import { LootExplorerLayout } from "./loot-explorer-layout";
import { LootExplorerStatus } from "./loot-explorer-status";
import { LootExplorerSummary } from "./loot-explorer-summary";
import { LootTable } from "./loot-table";

export function KaruakCeremonyExplorer({ selectedType }: { selectedType?: string }) {
  const [items, setItems] = useState<KaruakCeremonyLootDocument[]>([]);
  const [status, setStatus] = useState<"loading" | "ready" | "error">("loading");

  useEffect(() => {
    let ignore = false;

    fetchLootExplorerItems<KaruakCeremonyLootDocument>("karuak-ceremony")
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
    return <LootExplorerStatus active="karuak-ceremony" message="Loading loot explorer data..." />;
  }

  if (status === "error") {
    return (
      <LootExplorerStatus active="karuak-ceremony" message="Failed to load loot explorer data." />
    );
  }

  return <KaruakCeremonyExplorerContent items={items} selectedType={selectedType} />;
}

export function KaruakCeremonyExplorerContent({
  items,
  selectedType,
}: {
  items: KaruakCeremonyLootDocument[];
  selectedType?: string;
}) {
  const boss = findKaruakBoss(selectedType, items);
  const item = items.find((candidate) => candidate.kind === boss.kind);

  const labels = new Map([
    ["bladefist-andaal", "Bladefist Andaal"],
    ["bearkeeper-lukor", "Bearkeeper Lukor"],
    ["bruteshield-murdos", "Bruteshield Murdos"],
    ["warmender-pache", "Warmender Pache"],
    ["solon-por", "Solon Por"],
  ]);

  return (
    <LootExplorerLayout active="karuak-ceremony">
      <LootExplorerFilters
        typeOptions={karuakBosses.map((option) => ({
          value: option.key,
          label: labels.get(option.key) ?? option.label,
        }))}
        selectedType={boss.key}
        showLevelFilter={false}
      />
      <LootExplorerSummary
        generatedAt={item?.refreshedAt}
        items={[{ label: "Results", value: item?.totals.results ?? 0 }]}
      />
      <div className="space-y-8">
        {item?.slots.map((slot) => (
          <section key={slot.slot} className="space-y-3">
            <Subheading level={3}>{`Slot ${slot.slot.toString()}`}</Subheading>
            <LootTable loot={slot.loot} />
          </section>
        ))}
      </div>
    </LootExplorerLayout>
  );
}
