"use client";

import { useExtracted } from "next-intl";
import { useEffect, useState } from "react";
import { Listbox, ListboxLabel, ListboxOption } from "@/components/ui/listbox";
import type { LootExplorerOption } from "@/lib/loot-explorer/catalog";

export function LootExplorerFilters({
  typeOptions,
  selectedType,
  levelOptions,
  selectedLevels,
  allowMultipleLevels = true,
  showLevelFilter = true,
}: {
  typeOptions: LootExplorerOption[];
  selectedType: string;
  levelOptions?: LootExplorerOption[];
  selectedLevels?: number[];
  allowMultipleLevels?: boolean;
  showLevelFilter?: boolean;
}) {
  const t = useExtracted();
  const [levels, setLevels] = useState<string[]>(selectedLevels?.map(String) ?? []);

  useEffect(() => {
    setLevels(selectedLevels?.map(String) ?? []);
  }, [selectedLevels]);

  const levelSummary = levels.length
    ? levels
        .map((level) => levelOptions?.find((option) => option.value === level)?.label ?? level)
        .join(", ")
    : t("All");

  return (
    <form className="space-y-4">
      <div className="grid gap-4 sm:grid-cols-2 lg:grid-cols-[minmax(12rem,18rem)_minmax(12rem,18rem)]">
        <label className="space-y-1.5">
          <span className="block font-medium text-sm/6 text-zinc-700 dark:text-zinc-200">
            {t("NPC")}
          </span>
          <select
            className="block w-full rounded-lg border border-zinc-950/10 bg-white px-3 py-2 text-sm/6 text-zinc-950 shadow-sm dark:border-white/10 dark:bg-white/5 dark:text-white"
            name="type"
            defaultValue={selectedType}
          >
            {typeOptions.map((option) => (
              <option key={option.value} value={option.value}>
                {option.label}
              </option>
            ))}
          </select>
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
                  <span
                    className={levels.length ? "block truncate" : "block truncate text-zinc-500"}
                  >
                    {levelSummary}
                  </span>
                )}
                value={levels}
              >
                {(levelOptions ?? []).map((option) => (
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
                  <span
                    className={levels.length ? "block truncate" : "block truncate text-zinc-500"}
                  >
                    {levelSummary}
                  </span>
                )}
                value={levels[0] ?? ""}
              >
                {(levelOptions ?? []).map((option) => (
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
