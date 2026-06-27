"use client";

import { cn } from "cnfast";
import { useExtracted } from "next-intl";
import { useEffect, useState } from "react";
import { Field, Label } from "@/components/ui/fieldset";
import { Input } from "@/components/ui/input";
import { Listbox, ListboxLabel, ListboxOption } from "@/components/ui/listbox";
import type { LootExplorerOption } from "@/lib/loot-explorer/catalog";

const allLevelValue = "__all__";

type PersonalLootFiltersProps = {
  typeOptions: LootExplorerOption[];
  selectedType: string;
  levelOptions?: LootExplorerOption[];
  levelOptionsByType?: Record<string, LootExplorerOption[]>;
  selectedLevels?: number[];
  allowMultipleLevels?: boolean;
  showLevelFilter?: boolean;
  startDate: string;
  endDate: string;
  minDate: string;
  maxDate: string;
};

export function PersonalLootFilters({
  typeOptions,
  selectedType,
  levelOptions,
  levelOptionsByType,
  selectedLevels,
  allowMultipleLevels = true,
  showLevelFilter = true,
  startDate,
  endDate,
  minDate,
  maxDate,
}: PersonalLootFiltersProps) {
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
  const levelValue = levels.length ? levels : [allLevelValue];
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
      return allowMultipleLevels ? validLevels : validLevels.slice(0, 1);
    });
  };

  const handleMultipleLevelChange = (nextLevels: string[]) => {
    if (nextLevels.includes(allLevelValue)) {
      if (!levels.includes(allLevelValue) && levels.length > 0) {
        setLevels([]);
        return;
      }

      const withoutAll = nextLevels.filter((level) => level !== allLevelValue);
      setLevels(withoutAll);
      return;
    }

    setLevels(nextLevels);
  };

  const handleSingleLevelChange = (level: string) => {
    setLevels(level === allLevelValue ? [] : [level]);
  };

  return (
    <form className="space-y-4">
      <div className="grid gap-4 md:grid-cols-2 xl:grid-cols-4">
        <Field className="space-y-1.5">
          <Label>{t("NPC")}</Label>
          <Listbox<string> aria-label={t("NPC")} onChange={handleTypeChange} value={type}>
            {typeOptions.map((option) => (
              <ListboxOption key={option.value} value={option.value}>
                <ListboxLabel>{option.label}</ListboxLabel>
              </ListboxOption>
            ))}
          </Listbox>
          <input name="type" type="hidden" value={type} />
        </Field>
        {showLevelFilter ? (
          <Field className="space-y-1.5">
            <Label>{t("Level")}</Label>
            {allowMultipleLevels ? (
              <Listbox<string>
                aria-label={t("Level")}
                multiple
                onChange={handleMultipleLevelChange}
                placeholder={t("All")}
                renderValue={() => (
                  <span className={cn("block truncate", !levels.length && "text-zinc-500")}>
                    {levelSummary}
                  </span>
                )}
                value={levelValue}
              >
                <ListboxOption value={allLevelValue}>
                  <ListboxLabel>{t("All")}</ListboxLabel>
                </ListboxOption>
                {effectiveLevelOptions.map((option) => (
                  <ListboxOption key={option.value} value={option.value}>
                    <ListboxLabel>{option.label}</ListboxLabel>
                  </ListboxOption>
                ))}
              </Listbox>
            ) : (
              <Listbox<string>
                aria-label={t("Level")}
                onChange={handleSingleLevelChange}
                placeholder={t("All")}
                renderValue={() => (
                  <span className={cn("block truncate", !levels.length && "text-zinc-500")}>
                    {levelSummary}
                  </span>
                )}
                value={levels[0] ?? allLevelValue}
              >
                <ListboxOption value={allLevelValue}>
                  <ListboxLabel>{t("All")}</ListboxLabel>
                </ListboxOption>
                {effectiveLevelOptions.map((option) => (
                  <ListboxOption key={option.value} value={option.value}>
                    <ListboxLabel>{option.label}</ListboxLabel>
                  </ListboxOption>
                ))}
              </Listbox>
            )}
            {levels.length ? <input name="level" type="hidden" value={levels.join(",")} /> : null}
          </Field>
        ) : null}
        <Field className="space-y-1.5">
          <Label htmlFor="loot-start-date">{t("Start date")}</Label>
          <Input
            id="loot-start-date"
            name="start"
            type="date"
            defaultValue={startDate}
            min={minDate}
            max={maxDate}
          />
        </Field>
        <Field className="space-y-1.5">
          <Label htmlFor="loot-end-date">{t("End date")}</Label>
          <Input
            id="loot-end-date"
            name="end"
            type="date"
            defaultValue={endDate}
            min={minDate}
            max={maxDate}
          />
        </Field>
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
