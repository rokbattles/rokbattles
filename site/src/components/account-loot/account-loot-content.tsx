import { getExtracted } from "next-intl/server";
import { LootCategoryList } from "@/components/account-loot/loot-category-list";
import { LootFiltersClient } from "@/components/account-loot/loot-filters-client";
import { LootReportSection } from "@/components/account-loot/loot-report-section";
import { LootTableSection } from "@/components/account-loot/loot-table-section";
import { Text } from "@/components/ui/text";
import { formatLocalDateInput } from "@/lib/datetime";
import type { LootCategoryKey, LootQueryResult } from "@/lib/types/loot";

type AccountLootContentProps = {
  data: LootQueryResult;
  selectedCategory: LootCategoryKey;
  datasetLocale?: string;
};

export async function AccountLootContent({
  data,
  selectedCategory,
  datasetLocale,
}: AccountLootContentProps) {
  const t = await getExtracted();
  const selectedCategoryData = data.categories[selectedCategory];
  const minDate = "2025-01-01";
  const maxDate = formatLocalDateInput(new Date());

  return (
    <div className="space-y-8">
      <Text>{t("See loot from Barbarian, Barbarian Fort, and Baulur reports.")}</Text>
      <LootFiltersClient
        category={selectedCategory}
        startDate={data.range.start}
        endDate={data.range.end}
        minDate={minDate}
        maxDate={maxDate}
      />
      <div className="grid gap-8 lg:grid-cols-[15rem_minmax(0,1fr)] lg:items-start">
        <LootCategoryList
          categories={data.categories}
          selectedCategory={selectedCategory}
          startDate={data.range.start}
          endDate={data.range.end}
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
