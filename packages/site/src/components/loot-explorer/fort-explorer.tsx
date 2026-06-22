"use client";

import { useExtracted, useLocale } from "next-intl";
import { useEffect, useState } from "react";
import { Subheading } from "@/components/ui/heading";
import { type BarbarianFortLootDocument, fetchLootExplorerItems } from "@/lib/loot-explorer/api";
import { findFortFamily, fortFamilies, levelOptions } from "@/lib/loot-explorer/catalog";
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

  const family = findFortFamily(selectedType, items);
  const familyItems = items.filter((item) => item.kind === family.kind);
  const levels = levelOptions(familyItems, (level) =>
    t("Level {level}", { level: level.toString() })
  );
  const activeLevels = selectedLevels.length
    ? selectedLevels.slice(0, 1)
    : levels.slice(0, 1).map((option) => Number(option.value));
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
  const familyLabels = new Map([
    ["barbarian-forts", t("Barbarian Forts")],
    ["marauder-encampments", t("Marauder Encampments")],
    ["mottes", t("Mottes")],
  ]);
  const familyLabel = familyLabels.get(family.key) ?? family.label;

  return (
    <LootExplorerLayout active="barbarian-forts">
      <LootExplorerFilters
        typeOptions={fortFamilies
          .filter((option) => items.some((item) => item.kind === option.kind))
          .map((option) => ({
            value: option.key,
            label: familyLabels.get(option.key) ?? option.label,
          }))}
        selectedType={family.key}
        levelOptions={levels}
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
              {t("{name} level {level}", { level: item.level.toString(), name: familyLabel })}
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
