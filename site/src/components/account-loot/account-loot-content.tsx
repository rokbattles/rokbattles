"use client";

import { useSearchParams } from "next/navigation";
import { useExtracted } from "next-intl";
import { useContext, useMemo, useState } from "react";
import { LootCategoryList } from "@/components/account-loot/loot-category-list";
import { LootErrorState } from "@/components/account-loot/loot-error-state";
import { LootFiltersClient } from "@/components/account-loot/loot-filters-client";
import { LootReportSection } from "@/components/account-loot/loot-report-section";
import { LootTableSection } from "@/components/account-loot/loot-table-section";
import { Text } from "@/components/ui/text";
import { useLoot } from "@/hooks/use-loot";
import { formatLocalDateInput } from "@/lib/datetime";
import { parseLootSearchParams } from "@/lib/loot/search-params";
import type { LootCategoryKey } from "@/lib/types/loot";
import { GovernorContext } from "@/providers/governor-context";

type AccountLootContentProps = {
  datasetLocale?: string;
};

export function AccountLootContent({ datasetLocale }: AccountLootContentProps) {
  const t = useExtracted();
  const searchParams = useSearchParams();
  const governorContext = useContext(GovernorContext);
  if (!governorContext) {
    throw new Error("My Loot page must be used within a GovernorProvider");
  }

  const parsed = useMemo(
    () =>
      parseLootSearchParams({
        start: searchParams.get("start") ?? undefined,
        end: searchParams.get("end") ?? undefined,
      }),
    [searchParams]
  );
  const [selectedCategory, setSelectedCategory] = useState<LootCategoryKey>("barbarian");
  const governorId = governorContext.activeGovernor?.governorId;
  const { data, error } = useLoot({
    governorId,
    startParam: parsed.startParam,
    endParam: parsed.endParam,
  });

  if (error) {
    return <LootErrorState />;
  }

  if (!data) {
    return null;
  }

  const selectedCategoryData = data.categories[selectedCategory];
  const minDate = "2025-01-01";
  const maxDate = formatLocalDateInput(new Date());

  return (
    <div className="space-y-8">
      <Text>{t("See loot from Barbarian, Barbarian Fort, and Baulur reports.")}</Text>
      <LootFiltersClient
        startDate={data.range.start}
        endDate={data.range.end}
        minDate={minDate}
        maxDate={maxDate}
      />
      <div className="grid gap-8 lg:grid-cols-[15rem_minmax(0,1fr)] lg:items-start">
        <LootCategoryList
          categories={data.categories}
          selectedCategory={selectedCategory}
          onSelectCategory={setSelectedCategory}
        />
        <section>
          <LootReportSection
            category={selectedCategoryData}
            rangeStart={data.range.start}
            rangeEnd={data.range.end}
          />
        </section>
      </div>
      <LootTableSection category={selectedCategoryData} datasetLocale={datasetLocale} />
    </div>
  );
}
