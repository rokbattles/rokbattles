"use client";

import { Field, Label } from "@/components/ui/Fieldset";
import { Input } from "@/components/ui/Input";
import { Listbox, ListboxOption } from "@/components/ui/Listbox";
import type { LoadoutGranularity } from "@/hooks/usePairings";

type PairingOption = {
  value: string;
  label: string;
};

type PairingsFiltersProps = {
  pairingOptions: PairingOption[];
  pairingValue: string | null;
  onPairingChange: (value: string | null) => void;
  pairingsLoading: boolean;
  loadoutGranularity: LoadoutGranularity;
  onGranularityChange: (value: LoadoutGranularity) => void;
  startDate: string;
  endDate: string;
  onStartDateChange: (value: string) => void;
  onEndDateChange: (value: string) => void;
};

export function PairingsFilters({
  pairingOptions,
  pairingValue,
  onPairingChange,
  pairingsLoading,
  loadoutGranularity,
  onGranularityChange,
  startDate,
  endDate,
  onStartDateChange,
  onEndDateChange,
}: PairingsFiltersProps) {
  return (
    <div className="grid gap-4 md:grid-cols-2 xl:grid-cols-4">
      <Field className="space-y-2">
        <Label>Pairing</Label>
        <Listbox
          aria-label="Pairing"
          value={pairingValue}
          onChange={onPairingChange}
          disabled={pairingsLoading || pairingOptions.length === 0}
        >
          {pairingOptions.map((option) => (
            <ListboxOption key={option.value} value={option.value}>
              {option.label}
            </ListboxOption>
          ))}
        </Listbox>
      </Field>
      <Field className="space-y-2">
        <Label>Loadout granularity</Label>
        <Listbox
          aria-label="Loadout granularity"
          value={loadoutGranularity}
          onChange={onGranularityChange}
        >
          <ListboxOption value="normalized">Normalized</ListboxOption>
          <ListboxOption value="exact">Exact</ListboxOption>
        </Listbox>
      </Field>
      <Field className="space-y-2">
        <Label htmlFor="pairings-start-date">Start date</Label>
        <Input
          id="pairings-start-date"
          type="date"
          value={startDate}
          onChange={(event) => onStartDateChange(event.target.value)}
        />
      </Field>
      <Field className="space-y-2">
        <Label htmlFor="pairings-end-date">End date</Label>
        <Input
          id="pairings-end-date"
          type="date"
          value={endDate}
          onChange={(event) => onEndDateChange(event.target.value)}
        />
      </Field>
    </div>
  );
}
