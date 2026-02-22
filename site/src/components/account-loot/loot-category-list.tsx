import { getExtracted } from "next-intl/server";
import {
  Sidebar,
  SidebarBody,
  SidebarItem,
  SidebarLabel,
  SidebarSection,
} from "@/components/ui/sidebar";
import { formatWholeNumber } from "@/lib/loot/format";
import type { LootCategoryAggregate, LootCategoryKey } from "@/lib/types/loot";

type LootCategoryListProps = {
  categories: Record<LootCategoryKey, LootCategoryAggregate>;
  selectedCategory: LootCategoryKey;
  startDate: string;
  endDate: string;
};

function buildCategoryHref(category: LootCategoryKey, startDate: string, endDate: string): string {
  const params = new URLSearchParams();
  params.set("category", category);
  params.set("start", startDate);
  params.set("end", endDate);

  return `/account/loot?${params.toString()}`;
}

export async function LootCategoryList({
  categories,
  selectedCategory,
  startDate,
  endDate,
}: LootCategoryListProps) {
  const t = await getExtracted();
  const options: Array<{ key: LootCategoryKey; label: string }> = [
    { key: "barbarian", label: t("Barbarians") },
    { key: "barbarianFort", label: t("Barbarian Fort") },
    { key: "baulur", label: t("Baulur") },
  ];

  return (
    <Sidebar className="h-fit">
      <SidebarBody className="p-0">
        <SidebarSection>
          {options.map((option) => {
            const categoryData = categories[option.key];
            const isSelected = option.key === selectedCategory;

            return (
              <SidebarItem
                key={option.key}
                href={buildCategoryHref(option.key, startDate, endDate)}
                current={isSelected}
                aria-current={isSelected ? "true" : undefined}
              >
                <SidebarLabel>{option.label}</SidebarLabel>
                <span className="ml-auto text-xs tabular-nums text-zinc-500 dark:text-zinc-400">
                  {formatWholeNumber(categoryData.reports)}
                </span>
              </SidebarItem>
            );
          })}
        </SidebarSection>
      </SidebarBody>
    </Sidebar>
  );
}
