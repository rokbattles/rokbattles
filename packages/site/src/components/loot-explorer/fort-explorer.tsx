"use client";

import { useExtracted, useLocale } from "next-intl";
import { type ReactNode, useEffect, useState } from "react";
import { Subheading } from "@/components/ui/heading";
import { GameTranslate } from "@/components/v1/game-translate";
import { type BarbarianFortLootDocument, fetchLootExplorerItems } from "@/lib/loot-explorer/api";
import {
  findFortFamily,
  fortFamilies,
  levelOptions,
  resolveActiveLevels,
} from "@/lib/loot-explorer/catalog";
import { formatRange } from "@/lib/loot-explorer/format";
import { LootExplorerFilters } from "./loot-explorer-filters";
import { LootExplorerLayout } from "./loot-explorer-layout";
import { LootExplorerStatus } from "./loot-explorer-status";
import { LootExplorerSummary } from "./loot-explorer-summary";
import { LootTable } from "./loot-table";

export function FortExplorer({
  selectedType,
  selectedLevels,
}: {
  selectedType?: string;
  selectedLevels: number[];
}) {
  const t = useExtracted();
  const locale = useLocale();
  const [items, setItems] = useState<BarbarianFortLootDocument[]>([]);
  const [status, setStatus] = useState<"loading" | "ready" | "error">("loading");

  useEffect(() => {
    let ignore = false;

    setStatus("loading");
    fetchLootExplorerItems<BarbarianFortLootDocument>("barbarian-forts")
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
    return (
      <LootExplorerStatus active="barbarian-forts" message={t("Loading loot explorer data...")} />
    );
  }

  if (status === "error") {
    return (
      <LootExplorerStatus
        active="barbarian-forts"
        message={t("Failed to load loot explorer data.")}
      />
    );
  }

  const familyLabels = new Map<string, ReactNode>([
    ["barbarian-forts", <GameTranslate value="LC_COMMON_SEARCH_PVE_BAR_FORT" />],
    ["marauder-encampments", <GameTranslate value="LC_COMMON_SEARCH_PVE_BAR_MARA_EN" />],
    ["mottes", t("Mottes")],
  ]);
  const formatLevel = (level: number) => t("Level {level}", { level: level.toString() });
  const availableFamilies = fortFamilies.filter((option) =>
    items.some((item) => item.kind === option.kind)
  );
  const levelOptionsByType = Object.fromEntries(
    availableFamilies.map((option) => [
      option.key,
      levelOptions(
        items.filter((item) => item.kind === option.kind),
        formatLevel
      ),
    ])
  );
  const family = findFortFamily(selectedType, items);
  const familyItems = items.filter((item) => item.kind === family.kind);
  const levels = levelOptionsByType[family.key] ?? [];
  const activeLevels = resolveActiveLevels({
    allowMultiple: false,
    options: levels,
    selectedLevels,
  });
  const visibleItems = familyItems
    .filter((item) => activeLevels.includes(item.level))
    .sort((left, right) => left.level - right.level);
  const totals = visibleItems.reduce(
    (acc, item) => ({
      results: acc.results + item.totals.results,
      apUsed: acc.apUsed + item.totals.apUsed,
      honor: acc.honor + item.totals.honorPointsGained,
    }),
    { results: 0, apUsed: 0, honor: 0 }
  );
  const generatedAt = visibleItems[0]?.refreshedAt ?? familyItems[0]?.refreshedAt;
  const familyLabel = familyLabels.get(family.key) ?? family.label;

  return (
    <LootExplorerLayout active="barbarian-forts">
      <LootExplorerFilters
        typeOptions={availableFamilies.map((option) => ({
          value: option.key,
          label: familyLabels.get(option.key) ?? option.label,
        }))}
        selectedType={family.key}
        levelOptions={levels}
        levelOptionsByType={levelOptionsByType}
        selectedLevels={activeLevels}
        allowMultipleLevels={false}
      />
      <LootExplorerSummary
        generatedAt={generatedAt}
        items={[
          { label: t("Results"), value: totals.results },
          { label: t("AP used"), value: totals.apUsed },
          { label: t("Honor gained"), value: totals.honor },
        ]}
      />
      <div className="space-y-10">
        {visibleItems.map((item) => (
          <section key={`${item.kind}:${item.level}`} className="space-y-5">
            <Subheading>
              {familyLabel} {t("Level {level}", { level: item.level.toString() })}
            </Subheading>
            {item.rewardTiers.map((tier) => (
              <div key={tier.tier} className="space-y-3">
                <div>
                  <div className="font-medium text-sm/6 text-zinc-950 dark:text-white">
                    {t("Reward Tier {tier}", { tier: tier.tier.toString() })}
                  </div>
                  <div className="text-sm/6 text-zinc-500 dark:text-zinc-400">
                    {t(
                      "This reward tier has been seen {count, plural, one {# time} other {# times}}, requiring damage {damage}.",
                      {
                        count: tier.results,
                        damage: formatRange(tier.damagePercentage, "%", t("n/a")),
                      }
                    )}
                  </div>
                </div>
                <LootTable loot={tier.loot} locale={locale} />
              </div>
            ))}
          </section>
        ))}
      </div>
    </LootExplorerLayout>
  );
}
