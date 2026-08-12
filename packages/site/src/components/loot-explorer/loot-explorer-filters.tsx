"use client";

import { cn } from "cnfast";
import { useExtracted } from "next-intl";
import { type ReactNode, useEffect, useState } from "react";
import { Listbox, ListboxLabel, ListboxOption } from "@/components/ui/listbox";
import type { LootExplorerOption } from "@/lib/loot-explorer/catalog";

export function LootExplorerFilters({
  typeOptions,
  selectedType,
  levelOptions,
  levelOptionsByType,
  selectedLevels,
  allowMultipleLevels = true,
  showLevelFilter = true,
}: {
  typeOptions: LootExplorerOption<ReactNode>[];
  selectedType: string;
  levelOptions?: LootExplorerOption[];
  levelOptionsByType?: Record<string, LootExplorerOption[]>;
  selectedLevels?: number[];
  allowMultipleLevels?: boolean;
  showLevelFilter?: boolean;
}) {
  const t = useExtracted();
  const [type, setType] = useState(selectedType);
  const [levels, setLevels] = useState<string[]>(selectedLevels?.map(String) ?? []);

  useEffect(() => {
    setType(selectedType);
  }, [selectedType]);

  useEffect(() => {
    setLevels(selectedLevels?.map(String) ?? []);
  }, [selectedLevels]);

  const effectiveLevelOptions = levelOptionsByType?.[type] ?? levelOptions ?? [];
  const levelSummary = levels.length
    ? levels
        .map(
          (level) => effectiveLevelOptions.find((option) => option.value === level)?.label ?? level
        )
        .join(", ")
    : t("All");

  const handleTypeChange = (nextType: string) => {
    setType(nextType);
    if (!showLevelFilter) {
      return;
    }

    const nextLevelOptions = levelOptionsByType?.[nextType] ?? levelOptions ?? [];
    const nextLevelValues = new Set(nextLevelOptions.map((option) => option.value));
    setLevels((currentLevels) => {
      const validLevels = currentLevels.filter((level) => nextLevelValues.has(level));
      if (validLevels.length > 0) {
        return allowMultipleLevels ? validLevels : validLevels.slice(0, 1);
      }

      const firstLevel = nextLevelOptions[0]?.value;
      return firstLevel ? [firstLevel] : [];
    });
  };

  return (
    <form className="space-y-4">
      <div className="grid gap-4 sm:grid-cols-2 lg:grid-cols-[minmax(12rem,18rem)_minmax(12rem,18rem)]">
        <label className="space-y-1.5">
          <span className="block font-medium text-sm/6 text-zinc-700 dark:text-zinc-200">
            {t("NPC")}
          </span>
          <Listbox<string> aria-label={t("NPC")} onChange={handleTypeChange} value={type}>
            {typeOptions.map((option) => (
              <ListboxOption key={option.value} value={option.value}>
                <ListboxLabel>{option.label}</ListboxLabel>
              </ListboxOption>
            ))}
          </Listbox>
          <input name="type" type="hidden" value={type} />
        </label>
        {showLevelFilter ? (
          <div className="space-y-1.5">
            <span className="block font-medium text-sm/6 text-zinc-700 dark:text-zinc-200">
              {t("Level")}
            </span>
            {allowMultipleLevels ? (
              <Listbox<string>
                aria-label={t("Level")}
                multiple
                onChange={setLevels}
                placeholder={t("All")}
                renderValue={() => (
                  <span className={cn("block truncate", !levels.length && "text-zinc-500")}>
                    {levelSummary}
                  </span>
                )}
                value={levels}
              >
                {effectiveLevelOptions.map((option) => (
                  <ListboxOption key={option.value} value={option.value}>
                    <ListboxLabel>{option.label}</ListboxLabel>
                  </ListboxOption>
                ))}
              </Listbox>
            ) : (
              <Listbox<string>
                aria-label={t("Level")}
                onChange={(level) => setLevels(level ? [level] : [])}
                placeholder={t("All")}
                renderValue={() => (
                  <span className={cn("block truncate", !levels.length && "text-zinc-500")}>
                    {levelSummary}
                  </span>
                )}
                value={levels[0] ?? ""}
              >
                {effectiveLevelOptions.map((option) => (
                  <ListboxOption key={option.value} value={option.value}>
                    <ListboxLabel>{option.label}</ListboxLabel>
                  </ListboxOption>
                ))}
              </Listbox>
            )}
            {levels.map((level) => (
              <input key={level} name="level" type="hidden" value={level} />
            ))}
          </div>
        ) : null}
      </div>
      <div>
        <button
          type="submit"
          className="inline-flex h-10 items-center justify-center rounded-lg bg-zinc-900 px-3 font-semibold text-sm text-white shadow-sm hover:bg-zinc-700 dark:bg-white dark:text-zinc-950 dark:hover:bg-zinc-200"
        >
          {t("Apply")}
        </button>
      </div>
    </form>
  );
}
