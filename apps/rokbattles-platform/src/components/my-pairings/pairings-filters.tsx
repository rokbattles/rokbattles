"use client";

import { useTranslations } from "next-intl";
import { Field, Label } from "@/components/ui/fieldset";
import { Input } from "@/components/ui/input";
import { Listbox, ListboxOption } from "@/components/ui/listbox";
import type { LoadoutGranularity } from "@/hooks/use-pairings";
import { formatLocalDateInput } from "@/lib/datetime";

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
  const t = useTranslations("pairings");
  const tTrends = useTranslations("trends");
  const minDate = "2025-01-01";
  const maxDate = formatLocalDateInput(new Date());

  return (
    <div className="grid gap-4 md:grid-cols-2 xl:grid-cols-4">
      <Field className="space-y-2">
        <Label>{tTrends("table.pairing")}</Label>
        <Listbox
          aria-label={tTrends("table.pairing")}
          disabled={pairingsLoading || pairingOptions.length === 0}
          onChange={onPairingChange}
          value={pairingValue}
        >
          {pairingOptions.map((option) => (
            <ListboxOption key={option.value} value={option.value}>
              {option.label}
            </ListboxOption>
          ))}
        </Listbox>
      </Field>
      <Field className="space-y-2">
        <Label>{t("filters.loadoutGranularity")}</Label>
        <Listbox
          aria-label={t("filters.loadoutGranularity")}
          onChange={onGranularityChange}
          value={loadoutGranularity}
        >
          <ListboxOption value="normalized">
            {t("filters.normalized")}
          </ListboxOption>
          <ListboxOption value="exact">{t("filters.exact")}</ListboxOption>
        </Listbox>
      </Field>
      <Field className="space-y-2">
        <Label htmlFor="pairings-start-date">{t("filters.startDate")}</Label>
        <Input
          id="pairings-start-date"
          max={maxDate}
          min={minDate}
          onChange={(event) => onStartDateChange(event.target.value)}
          type="date"
          value={startDate}
        />
      </Field>
      <Field className="space-y-2">
        <Label htmlFor="pairings-end-date">{t("filters.endDate")}</Label>
        <Input
          id="pairings-end-date"
          max={maxDate}
          min={minDate}
          onChange={(event) => onEndDateChange(event.target.value)}
          type="date"
          value={endDate}
        />
      </Field>
    </div>
  );
}
