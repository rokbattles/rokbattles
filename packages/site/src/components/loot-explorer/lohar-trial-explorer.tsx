"use client";

import { useExtracted, useLocale } from "next-intl";
import { useEffect, useState } from "react";
import { Subheading } from "@/components/ui/heading";
import { fetchLootExplorerItems, type LoharTrialLootDocument } from "@/lib/loot-explorer/api";
import { findLoharNpc, loharNpcs } from "@/lib/loot-explorer/catalog";
import { LootExplorerFilters } from "./loot-explorer-filters";
import { LootExplorerLayout } from "./loot-explorer-layout";
import { LootExplorerStatus } from "./loot-explorer-status";
import { LootExplorerSummary } from "./loot-explorer-summary";
import { LootTable } from "./loot-table";

export function LoharTrialExplorer({ selectedType }: { selectedType?: string }) {
  const t = useExtracted();
  const [items, setItems] = useState<LoharTrialLootDocument[]>([]);
  const [status, setStatus] = useState<"loading" | "ready" | "error">("loading");

  useEffect(() => {
    let ignore = false;

    fetchLootExplorerItems<LoharTrialLootDocument>("lohars-trial")
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
      <LootExplorerStatus active="lohars-trial" message={t("Loading loot explorer data...")} />
    );
  }

  if (status === "error") {
    return (
      <LootExplorerStatus active="lohars-trial" message={t("Failed to load loot explorer data.")} />
    );
  }

  return <LoharTrialExplorerContent items={items} selectedType={selectedType} />;
}

export function LoharTrialExplorerContent({
  items,
  selectedType,
}: {
  items: LoharTrialLootDocument[];
  selectedType?: string;
}) {
  const t = useExtracted();
  const locale = useLocale();

  const boss = findLoharNpc(selectedType, items);
  const item = items.find((candidate) => candidate.kind === boss.kind);

  const labels = new Map([
    ["junior-lohar", t("Junior Lohar")],
    ["dauntless-lohar", t("Dauntless Lohar")],
  ]);

  return (
    <LootExplorerLayout active="lohars-trial">
      <LootExplorerFilters
        typeOptions={loharNpcs.map((option) => ({
          value: option.key,
          label: labels.get(option.key) ?? option.label,
        }))}
        selectedType={boss.key}
        showLevelFilter={false}
      />
      <LootExplorerSummary
        generatedAt={item?.refreshedAt}
        items={[
          { label: t("Results"), value: item?.totals.results ?? 0 },
          { label: t("AP used"), value: item?.totals.apUsed ?? 0 },
        ]}
      />
      {item && item.totals.unmatchedResults > 0 ? (
        <p className="rounded-lg px-4 py-3 text-sm/6 bg-amber-500/10 text-amber-200">
          {t(
            "Rates exclude {count, plural, one {# reward result} other {# reward results}} that could not be assigned to the five slots.",
            { count: item.totals.unmatchedResults }
          )}
        </p>
      ) : null}
      <div className="space-y-8">
        {item?.slots.map((slot) => (
          <section key={slot.slot} className="space-y-3">
            <Subheading level={3}>{t("Slot {slot}", { slot: slot.slot.toString() })}</Subheading>
            <LootTable
              loot={slot.loot}
              locale={locale}
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
