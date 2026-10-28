"use client";

import { useEffect, useState } from "react";
import { Subheading } from "@/components/ui/heading";
import { type BaulurLootDocument, fetchLootExplorerItems } from "@/lib/loot-explorer/api";
import { baulurFamilies, findBaulurFamily } from "@/lib/loot-explorer/catalog";
import { BaulurRolls } from "./baulur-rolls";
import { LootExplorerFilters } from "./loot-explorer-filters";
import { LootExplorerLayout } from "./loot-explorer-layout";
import { LootExplorerStatus } from "./loot-explorer-status";
import { LootExplorerSummary } from "./loot-explorer-summary";
import { LootTable } from "./loot-table";

const numberFormatter = new Intl.NumberFormat("en-US");

export function BaulurExplorer({ selectedType }: { selectedType?: string }) {
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
    return <LootExplorerStatus active="baulurs" message="Loading loot explorer data..." />;
  }

  if (status === "error") {
    return <LootExplorerStatus active="baulurs" message="Failed to load loot explorer data." />;
  }

  return <BaulurExplorerContent items={items} selectedType={selectedType} />;
}

export function BaulurExplorerContent({
  items,
  selectedType,
}: {
  items: BaulurLootDocument[];
  selectedType?: string;
}) {
  const family = findBaulurFamily(selectedType, items);
  const item = items.find((candidate) => candidate.kind === family.kind);
  const familyLabels = new Map([
    ["ironhand-baulur", "Ironhand Baulur"],
    ["miser-khaolak", "Miser Khaolak"],
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
        items={[{ label: "Results", value: item?.totals.results ?? 0 }]}
      />
      {item ? (
        <div className="space-y-10">
          <section className="space-y-3">
            <div>
              <Subheading>1% damage or below</Subheading>
              <div className="text-sm/6 text-zinc-400">
                {`This has been seen ${
                  Math.abs(item.resourcePool.results) === 1
                    ? `${numberFormatter.format(item.resourcePool.results)} time`
                    : `${numberFormatter.format(item.resourcePool.results)} times`
                }.`}
              </div>
            </div>
            <LootTable loot={item.resourcePool.loot} />
          </section>
          <BaulurRolls pool={item.rollPool} />
        </div>
      ) : null}
    </LootExplorerLayout>
  );
}
