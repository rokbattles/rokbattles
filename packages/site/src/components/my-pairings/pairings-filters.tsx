"use client";

import { usePathname, useRouter, useSearchParams } from "next/navigation";
import { Listbox, ListboxOption } from "@/components/ui/listbox";
import { ACTIVITY_LABELS, BATTLE_LABELS, readExclusions } from "@/lib/pairings/filters";

export function PairingsFilters({ compact = false }: { compact?: boolean }) {
  const params = useSearchParams();
  const pathname = usePathname();
  const router = useRouter();

  function update(key: string, values: string[]) {
    const next = new URLSearchParams(params.toString());
    if (values.length) next.set(key, values.join(","));
    else next.delete(key);
    next.delete("loadout");
    router.replace(`${pathname}?${next}`, { scroll: false });
  }

  return (
    <div className="flex flex-wrap gap-3">
      <IncludedFilter
        compact={compact}
        label="Activities"
        allLabel="All activities"
        excluded={readExclusions(params.get("excludeActivities"), ACTIVITY_LABELS)}
        labels={ACTIVITY_LABELS}
        onChange={(values) => update("excludeActivities", values)}
      />
      <IncludedFilter
        compact={compact}
        label="Battle types"
        allLabel="All battle types"
        excluded={readExclusions(params.get("excludeBattles"), BATTLE_LABELS)}
        labels={BATTLE_LABELS}
        onChange={(values) => update("excludeBattles", values)}
      />
    </div>
  );
}

function IncludedFilter<T extends string>({
  label,
  compact,
  allLabel,
  excluded,
  labels,
  onChange,
}: {
  label: string;
  compact: boolean;
  allLabel: string;
  excluded: T[];
  labels: Record<T, string>;
  onChange: (values: T[]) => void;
}) {
  const keys = Object.keys(labels) as T[];
  const excludedSet = new Set(excluded);
  const included = keys.filter((key) => !excludedSet.has(key));
  const summary =
    included.length === keys.length
      ? allLabel
      : included.length === 0
        ? `No ${label.toLowerCase()}`
        : included.length === 1
          ? labels[included[0]]
          : `${included.length} ${label.toLowerCase()}`;

  return (
    <div className={compact ? "w-38 max-w-full" : "w-44 max-w-full"}>
      <Listbox<T>
        aria-label={label}
        multiple
        value={included}
        onChange={(values) => {
          const selected = new Set(values);
          onChange(keys.filter((key) => !selected.has(key)));
        }}
        renderValue={() => <span className="block truncate">{summary}</span>}
      >
        {keys.map((key) => (
          <ListboxOption key={key} value={key}>
            {labels[key]}
          </ListboxOption>
        ))}
      </Listbox>
    </div>
  );
}
