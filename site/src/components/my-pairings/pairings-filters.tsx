"use client";

import { useExtracted } from "next-intl";
import { Field, Label } from "@/components/ui/fieldset";
import { Input } from "@/components/ui/input";
import { Listbox, ListboxOption } from "@/components/ui/listbox";
import { formatLocalDateInput } from "@/lib/datetime";
import type { LoadoutGranularity } from "@/lib/pairings";

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
  const t = useExtracted();
  const minDate = "2025-01-01";
  const maxDate = formatLocalDateInput(new Date());

  return (
    <div className="grid gap-4 md:grid-cols-2 xl:grid-cols-4">
      <Field className="space-y-2">
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
    </div>
  );
}
