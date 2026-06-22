"use client";

import { useExtracted, useLocale } from "next-intl";
import { useEffect, useState } from "react";
import { Subheading } from "@/components/ui/heading";
import { type BarbarianLootDocument, fetchLootExplorerItems } from "@/lib/loot-explorer/api";
import {
  barbarianFamilies,
  findBarbarianFamily,
  levelOptions,
  resolveActiveLevels,
} from "@/lib/loot-explorer/catalog";
import { LootExplorerFilters } from "./loot-explorer-filters";
import { LootExplorerLayout } from "./loot-explorer-layout";
import { LootExplorerStatus } from "./loot-explorer-status";
import { LootExplorerSummary } from "./loot-explorer-summary";
import { LootTable } from "./loot-table";

export function BarbarianExplorer({
  selectedType,
  selectedLevels,
}: {
  selectedType?: string;
  selectedLevels: number[];
}) {
  const t = useExtracted();
  const locale = useLocale();
  const [items, setItems] = useState<BarbarianLootDocument[]>([]);
  const [status, setStatus] = useState<"loading" | "ready" | "error">("loading");

  useEffect(() => {
    let ignore = false;

    setStatus("loading");
    fetchLootExplorerItems<BarbarianLootDocument>("barbarians")
      .then((response) => {
        if (ignore) {
          return;
        }
        setItems(response.items);
        setStatus("ready");
      })
      .catch(() => {
        if (!ignore) {
          setStatus("error");
        }
      });

    return () => {
      ignore = true;
    };
  }, []);

  if (status === "loading") {
    return <LootExplorerStatus active="barbarians" message={t("Loading loot explorer data...")} />;
  }

  if (status === "error") {
    return (
      <LootExplorerStatus active="barbarians" message={t("Failed to load loot explorer data.")} />
    );
  }

  const familyLabels = new Map([
    ["barbarians", t("Barbarians")],
    ["barbarian-wolf-tamers-pack-striders", t("Barbarian Wolf Tamers/Pack Striders")],
    ["barbarian-bone-archers-heavy-archers", t("Barbarian Bone Archers/Heavy Archers")],
    ["barbarian-beast-riders-blitz-hunters", t("Barbarian Beast Riders/Blitz Hunters")],
    ["english-soldiers", t("English Soldiers")],
    ["marauders", t("Marauders")],
  ]);
  const formatLevel = (level: number) => t("Level {level}", { level: level.toString() });
  const availableFamilies = barbarianFamilies.filter((option) => items.some(option.matches));
  const levelOptionsByType = Object.fromEntries(
    availableFamilies.map((option) => [
      option.key,
      levelOptions(items.filter(option.matches), formatLevel),
    ])
  );
  const family = findBarbarianFamily(selectedType, items);
  const familyItems = items.filter(family.matches);
  const levels = levelOptionsByType[family.key] ?? [];
  const activeLevels = resolveActiveLevels({
    allowMultiple: true,
    options: levels,
    selectedLevels,
  });
  const visibleItems = familyItems
    .filter((item) => activeLevels.includes(item.level))
    .sort((left, right) => left.level - right.level || left.kind - right.kind);
  const totals = visibleItems.reduce(
    (acc, item) => ({
      results: acc.results + item.totals.results,
      apUsed: acc.apUsed + item.totals.apUsed,
      honor: acc.honor + item.totals.honorPointsGained,
      xp: acc.xp + item.totals.xpGained,
    }),
    { results: 0, apUsed: 0, honor: 0, xp: 0 }
  );
  const generatedAt = visibleItems[0]?.refreshedAt ?? familyItems[0]?.refreshedAt;
  const familyLabel = familyLabels.get(family.key) ?? family.label;

  return (
    <LootExplorerLayout active="barbarians">
      <LootExplorerFilters
        typeOptions={availableFamilies.map((option) => ({
          value: option.key,
          label: familyLabels.get(option.key) ?? option.label,
        }))}
        selectedType={family.key}
        levelOptions={levels}
        levelOptionsByType={levelOptionsByType}
        selectedLevels={activeLevels}
      />
      <LootExplorerSummary
        generatedAt={generatedAt}
        items={[
          { label: t("Results"), value: totals.results },
          { label: t("AP used"), value: totals.apUsed },
          { label: t("Honor gained"), value: totals.honor },
          { label: t("XP gained"), value: totals.xp },
        ]}
      />
      <div className="space-y-8">
        {visibleItems.map((item) => (
          <section key={`${item.kind}:${item.level}`} className="space-y-3">
            <Subheading>
              {t("{name} level {level}", { level: item.level.toString(), name: familyLabel })}
            </Subheading>
            <LootTable loot={item.loot} locale={locale} />
          </section>
        ))}
      </div>
    </LootExplorerLayout>
  );
}
