"use client";

import { useExtracted, useLocale } from "next-intl";
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
  const t = useExtracted();
  const locale = useLocale();
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
    return (
      <LootExplorerStatus active="karuak-ceremony" message={t("Loading loot explorer data...")} />
    );
  }
  if (status === "error") {
    return (
      <LootExplorerStatus
        active="karuak-ceremony"
        message={t("Failed to load loot explorer data.")}
      />
    );
  }

  const boss = findKaruakBoss(selectedType, items);
  const item = items.find((candidate) => candidate.kind === boss.kind);
  const labels = new Map([
    ["bladefist-andaal", t("Bladefist Andaal")],
    ["bearkeeper-lukor", t("Bearkeeper Lukor")],
    ["bruteshield-murdos", t("Bruteshield Murdos")],
    ["warmender-pache", t("Warmender Pache")],
    ["solon-por", t("Solon Por")],
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
        items={[{ label: t("Results"), value: item?.totals.results ?? 0 }]}
      />
      <section className="space-y-3">
        <Subheading>{labels.get(boss.key) ?? boss.label}</Subheading>
        <LootTable loot={item?.loot ?? []} locale={locale} />
      </section>
    </LootExplorerLayout>
  );
}
