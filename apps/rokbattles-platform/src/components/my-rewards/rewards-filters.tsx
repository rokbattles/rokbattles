"use client";

import { useTranslations } from "next-intl";
import { Field, Label } from "@/components/ui/fieldset";
import { Input } from "@/components/ui/input";
import { formatLocalDateInput } from "@/lib/datetime";

type RewardsFiltersProps = {
  startDate: string;
  endDate: string;
  onStartDateChange: (value: string) => void;
  onEndDateChange: (value: string) => void;
};

export function RewardsFilters({
  startDate,
  endDate,
  onStartDateChange,
  onEndDateChange,
}: RewardsFiltersProps) {
  const tPairings = useTranslations("pairings");
  const minDate = "2025-01-01";
  const maxDate = formatLocalDateInput(new Date());

  return (
    <div className="grid gap-4 md:grid-cols-2">
      <Field className="space-y-2">
        <Label htmlFor="rewards-start-date">
          {tPairings("filters.startDate")}
        </Label>
        <Input
          id="rewards-start-date"
          max={maxDate}
          min={minDate}
          onChange={(event) => onStartDateChange(event.target.value)}
          type="date"
          value={startDate}
        />
      </Field>
      <Field className="space-y-2">
        <Label htmlFor="rewards-end-date">{tPairings("filters.endDate")}</Label>
        <Input
          id="rewards-end-date"
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
