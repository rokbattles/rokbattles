"use client";

import { useExtracted } from "next-intl";
import { Field, Label } from "@/components/ui/fieldset";
import { Input } from "@/components/ui/input";
import { Listbox, ListboxLabel, ListboxOption } from "@/components/ui/listbox";
import { formatLocalDateInput } from "@/lib/datetime";
import {
  formatExcludedPairingsReportTypes,
  type LoadoutGranularity,
  type PairingsReportType,
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
  excludedReportTypes: PairingsReportType[];
  onExcludedReportTypesChange: (value: PairingsReportType[]) => void;
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
  excludedReportTypes,
  onExcludedReportTypesChange,
}: PairingsFiltersProps) {
  const t = useExtracted();
  const minDate = "2025-01-01";
  const maxDate = formatLocalDateInput(new Date());
  const excludeTypeLabels: Record<PairingsReportType, string> = {
    ark: t("Ark of Osiris"),
    home: t("Home"),
    kvk: t("KVK"),
    strife: t("Supreme Strife"),
  };
  const excludedTypeSummary = formatExcludedPairingsReportTypes(
    excludedReportTypes,
    excludeTypeLabels
  );

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
        <Label>{t("Exclude battles")}</Label>
        <Listbox<PairingsReportType>
          aria-label={t("Exclude battles")}
          value={excludedReportTypes}
          onChange={onExcludedReportTypesChange}
          multiple
          placeholder={t("None")}
          renderValue={() =>
            excludedTypeSummary ? (
              <span className="block truncate">{excludedTypeSummary}</span>
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
    </div>
  );
}
