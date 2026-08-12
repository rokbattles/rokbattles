"use client";

import { useExtracted, useLocale } from "next-intl";
import { type ReactNode, useEffect, useState } from "react";
import { Subheading } from "@/components/ui/heading";
import { GameTranslate } from "@/components/v1/game-translate";
import { type BaulurLootDocument, fetchLootExplorerItems } from "@/lib/loot-explorer/api";
import { baulurFamilies, findBaulurFamily } from "@/lib/loot-explorer/catalog";
import { LootExplorerFilters } from "./loot-explorer-filters";
import { LootExplorerLayout } from "./loot-explorer-layout";
import { LootExplorerStatus } from "./loot-explorer-status";
import { LootExplorerSummary } from "./loot-explorer-summary";
import { LootTable } from "./loot-table";

export function BaulurExplorer({ selectedType }: { selectedType?: string }) {
  const t = useExtracted();
  const locale = useLocale();
  const [items, setItems] = useState<BaulurLootDocument[]>([]);
  const [status, setStatus] = useState<"loading" | "ready" | "error">("loading");

  useEffect(() => {
    let ignore = false;

    setStatus("loading");
    fetchLootExplorerItems<BaulurLootDocument>("baulurs")
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
    return <LootExplorerStatus active="baulurs" message={t("Loading loot explorer data...")} />;
  }

  if (status === "error") {
    return (
      <LootExplorerStatus active="baulurs" message={t("Failed to load loot explorer data.")} />
    );
  }

  const family = findBaulurFamily(selectedType, items);
  const item = items.find((candidate) => candidate.kind === family.kind);
  const familyLabels = new Map<string, ReactNode>([
    ["ironhand-baulur", <GameTranslate value="LC_COMMON_SMALL_CAYON_BOSS_NAME" />],
    ["miser-khaolak", <GameTranslate value="LC_COMMON_SMALL_CAYON_RARE_NAME" />],
  ]);
  const poolLabels = new Map([
    [0, t("Under 1% damage")],
    [1, t("1%-100% damage")],
  ]);

  return (
    <LootExplorerLayout active="baulurs">
      <LootExplorerFilters
        typeOptions={baulurFamilies
          .filter((option) => items.some((candidate) => candidate.kind === option.kind))
          .map((option) => ({
            value: option.key,
            label: familyLabels.get(option.key) ?? option.label,
          }))}
        selectedType={family.key}
        showLevelFilter={false}
      />
      <LootExplorerSummary
        generatedAt={item?.refreshedAt}
        items={[{ label: t("Results"), value: item?.totals.results ?? 0 }]}
      />
      <div className="space-y-8">
        {item?.lootPools.map((pool) => (
          <section key={pool.pool} className="space-y-3">
            <div>
              <Subheading>
                {poolLabels.get(pool.pool) ??
                  t("Damage pool {pool}", { pool: pool.pool.toString() })}
              </Subheading>
              <div className="text-sm/6 text-zinc-500 dark:text-zinc-400">
                {t("This has been seen {count, plural, one {# time} other {# times}}.", {
                  count: pool.results,
                })}
              </div>
            </div>
            <LootTable loot={pool.loot} locale={locale} />
          </section>
        ))}
      </div>
    </LootExplorerLayout>
  );
}
