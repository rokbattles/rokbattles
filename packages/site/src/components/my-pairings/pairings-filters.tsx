"use client";

import { useExtracted } from "next-intl";
import { Field, Label } from "@/components/ui/fieldset";
import { Input } from "@/components/ui/input";
import { Listbox, ListboxLabel, ListboxOption } from "@/components/ui/listbox";
import { formatLocalDateInput } from "@/lib/datetime";
import {
  formatExcludedPairingsFilters,
  type LoadoutGranularity,
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
  loadoutGranularity: LoadoutGranularity;
  onGranularityChange: (value: LoadoutGranularity) => void;
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
  loadoutGranularity,
  onGranularityChange,
  startDate,
  endDate,
  onStartDateChange,
  onEndDateChange,
  excludedActivities,
  onExcludedActivitiesChange,
  excludedBattles,
  onExcludedBattlesChange,
}: PairingsFiltersProps) {
  const t = useExtracted();
  const minDate = "2025-01-01";
  const maxDate = formatLocalDateInput(new Date());
  const activityLabels: Record<PairingsActivity, string> = {
    ark: t("Ark of Osiris"),
    home: t("Home"),
    kvk: t("KVK"),
    strife: t("Supreme Strife"),
  };
  const battleLabels: Record<PairingsBattleType, string> = {
    "open-field": t("Open Field"),
    swarming: t("Swarming"),
    rally: t("Rally"),
    garrison: t("Garrison"),
  };
  const excludedActivitySummary = formatExcludedPairingsFilters(excludedActivities, activityLabels);
  const excludedBattleSummary = formatExcludedPairingsFilters(excludedBattles, battleLabels);

  return (
    <div className="grid gap-4 md:grid-cols-2 xl:grid-cols-4">
      <Field className="space-y-2 xl:col-span-2">
        <Label>{t("Pairing")}</Label>
        <Listbox
          aria-label={t("Pairing")}
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
        <Label htmlFor="pairings-start-date">{t("Start date")}</Label>
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
        <Label htmlFor="pairings-end-date">{t("End date")}</Label>
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
        <Label>{t("Loadout granularity")}</Label>
        <Listbox
          aria-label={t("Loadout granularity")}
          value={loadoutGranularity}
          onChange={onGranularityChange}
        >
          <ListboxOption value="simplified">{t("Simplified")}</ListboxOption>
          <ListboxOption value="exact">{t("Exact")}</ListboxOption>
        </Listbox>
      </Field>
      <Field className="space-y-2">
        <Label>{t("Exclude activities")}</Label>
        <Listbox<PairingsActivity>
          aria-label={t("Exclude activities")}
          value={excludedActivities}
          onChange={onExcludedActivitiesChange}
          multiple
          placeholder={t("None")}
          renderValue={() =>
            excludedActivitySummary ? (
              <span className="block truncate">{excludedActivitySummary}</span>
            ) : (
              <span className="block truncate text-zinc-500">{t("None")}</span>
            )
          }
        >
          <ListboxOption value="ark">
            <ListboxLabel>{t("Ark of Osiris")}</ListboxLabel>
          </ListboxOption>
          <ListboxOption value="kvk">
            <ListboxLabel>{t("KVK")}</ListboxLabel>
          </ListboxOption>
          <ListboxOption value="strife">
            <ListboxLabel>{t("Supreme Strife")}</ListboxLabel>
          </ListboxOption>
          <ListboxOption value="home">
            <ListboxLabel>{t("Home")}</ListboxLabel>
          </ListboxOption>
        </Listbox>
      </Field>
      <Field className="space-y-2">
        <Label>{t("Exclude battles")}</Label>
        <Listbox<PairingsBattleType>
          aria-label={t("Exclude battles")}
          value={excludedBattles}
          onChange={onExcludedBattlesChange}
          multiple
          placeholder={t("None")}
          renderValue={() =>
            excludedBattleSummary ? (
              <span className="block truncate">{excludedBattleSummary}</span>
            ) : (
              <span className="block truncate text-zinc-500">{t("None")}</span>
            )
          }
        >
          <ListboxOption value="open-field">
            <ListboxLabel>{t("Open Field")}</ListboxLabel>
          </ListboxOption>
          <ListboxOption value="swarming">
            <ListboxLabel>{t("Swarming")}</ListboxLabel>
          </ListboxOption>
          <ListboxOption value="rally">
            <ListboxLabel>{t("Rally")}</ListboxLabel>
          </ListboxOption>
          <ListboxOption value="garrison">
            <ListboxLabel>{t("Garrison")}</ListboxLabel>
          </ListboxOption>
        </Listbox>
      </Field>
    </div>
  );
}
