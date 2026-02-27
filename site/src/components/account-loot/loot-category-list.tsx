"use client";

import { useExtracted } from "next-intl";
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
  onSelectCategory: (category: LootCategoryKey) => void;
};

export function LootCategoryList({
  categories,
  selectedCategory,
  onSelectCategory,
}: LootCategoryListProps) {
  const t = useExtracted();
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
                current={isSelected}
                aria-current={isSelected ? "true" : undefined}
                onClick={() => onSelectCategory(option.key)}
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
