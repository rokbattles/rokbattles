"use client";

import { Field, Label } from "@/components/ui/fieldset";
import { Input } from "@/components/ui/input";
import { Listbox, ListboxLabel, ListboxOption } from "@/components/ui/listbox";
import { formatLocalDateInput } from "@/lib/datetime";
import {
  formatExcludedPairingsFilters,
  type PairingsActivity,
  type PairingsBattleType,
} from "@/lib/pairings";

type PairingOption = {
  value: string;
  label: string;
};

type PairingsFiltersProps = {
  pairingOptions: PairingOption[];
  pairingValue: string | null;
  onPairingChange: (value: string | null) => void;
  pairingsLoading: boolean;
  startDate: string;
  endDate: string;
  onStartDateChange: (value: string) => void;
  onEndDateChange: (value: string) => void;
  excludedActivities: PairingsActivity[];
  onExcludedActivitiesChange: (value: PairingsActivity[]) => void;
  excludedBattles: PairingsBattleType[];
  onExcludedBattlesChange: (value: PairingsBattleType[]) => void;
};

export function PairingsFilters({
  pairingOptions,
  pairingValue,
  onPairingChange,
  pairingsLoading,
  startDate,
  endDate,
  onStartDateChange,
  onEndDateChange,
  excludedActivities,
  onExcludedActivitiesChange,
  excludedBattles,
  onExcludedBattlesChange,
}: PairingsFiltersProps) {
  const minDate = "2025-01-01";
  const maxDate = formatLocalDateInput(new Date());
  const activityLabels: Record<PairingsActivity, string> = {
    ark: "Ark of Osiris",
    home: "Home",
    kvk: "KVK",
    strife: "Supreme Strife",
  };
  const battleLabels: Record<PairingsBattleType, string> = {
    "open-field": "Open Field",
    swarming: "Swarming",
    rally: "Rally",
    garrison: "Garrison",
  };
  const excludedActivitySummary = formatExcludedPairingsFilters(excludedActivities, activityLabels);
  const excludedBattleSummary = formatExcludedPairingsFilters(excludedBattles, battleLabels);

  return (
    <div className="grid gap-4 md:grid-cols-2 xl:grid-cols-4">
      <Field className="space-y-2 xl:col-span-2">
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
        <Label htmlFor="pairings-start-date">Start date</Label>
        <Input
          id="pairings-start-date"
          type="date"
          value={startDate}
          min={minDate}
          max={maxDate}
          onChange={(event) => onStartDateChange(event.target.value)}
        />
      </Field>
      <Field className="space-y-2">
        <Label htmlFor="pairings-end-date">End date</Label>
        <Input
          id="pairings-end-date"
          type="date"
          value={endDate}
          min={minDate}
          max={maxDate}
          onChange={(event) => onEndDateChange(event.target.value)}
        />
      </Field>
      <Field className="space-y-2">
        <Label>Exclude activities</Label>
        <Listbox<PairingsActivity>
          aria-label="Exclude activities"
          value={excludedActivities}
          onChange={onExcludedActivitiesChange}
          multiple
          placeholder="None"
          renderValue={() =>
            excludedActivitySummary ? (
              <span className="block truncate">{excludedActivitySummary}</span>
            ) : (
              <span className="block truncate text-zinc-500">None</span>
            )
          }
        >
          <ListboxOption value="ark">
            <ListboxLabel>Ark of Osiris</ListboxLabel>
          </ListboxOption>
          <ListboxOption value="kvk">
            <ListboxLabel>KVK</ListboxLabel>
          </ListboxOption>
          <ListboxOption value="strife">
            <ListboxLabel>Supreme Strife</ListboxLabel>
          </ListboxOption>
          <ListboxOption value="home">
            <ListboxLabel>Home</ListboxLabel>
          </ListboxOption>
        </Listbox>
      </Field>
      <Field className="space-y-2">
        <Label>Exclude battles</Label>
        <Listbox<PairingsBattleType>
          aria-label="Exclude battles"
          value={excludedBattles}
          onChange={onExcludedBattlesChange}
          multiple
          placeholder="None"
          renderValue={() =>
            excludedBattleSummary ? (
              <span className="block truncate">{excludedBattleSummary}</span>
            ) : (
              <span className="block truncate text-zinc-500">None</span>
            )
          }
        >
          <ListboxOption value="open-field">
            <ListboxLabel>Open Field</ListboxLabel>
          </ListboxOption>
          <ListboxOption value="swarming">
            <ListboxLabel>Swarming</ListboxLabel>
          </ListboxOption>
          <ListboxOption value="rally">
            <ListboxLabel>Rally</ListboxLabel>
          </ListboxOption>
          <ListboxOption value="garrison">
            <ListboxLabel>Garrison</ListboxLabel>
          </ListboxOption>
        </Listbox>
      </Field>
    </div>
  );
}
