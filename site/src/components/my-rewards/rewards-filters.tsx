"use client";

import { useExtracted } from "next-intl";
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
  const t = useExtracted();
  const minDate = "2025-01-01";
  const maxDate = formatLocalDateInput(new Date());

  return (
    <div className="grid gap-4 md:grid-cols-2">
      <Field className="space-y-2">
        <Label htmlFor="rewards-start-date">{t("Start date")}</Label>
        <Input
          id="rewards-start-date"
          type="date"
          value={startDate}
          min={minDate}
          max={maxDate}
          onChange={(event) => onStartDateChange(event.target.value)}
        />
      </Field>
      <Field className="space-y-2">
        <Label htmlFor="rewards-end-date">{t("End date")}</Label>
        <Input
          id="rewards-end-date"
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
