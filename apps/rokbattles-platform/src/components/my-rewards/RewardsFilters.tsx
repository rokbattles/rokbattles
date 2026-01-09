"use client";

import { useTranslations } from "next-intl";
import { Field, Label } from "@/components/ui/Fieldset";
import { Input } from "@/components/ui/Input";

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

  return (
    <div className="grid gap-4 md:grid-cols-2">
      <Field className="space-y-2">
        <Label htmlFor="rewards-start-date">{tPairings("filters.startDate")}</Label>
        <Input
          id="rewards-start-date"
          type="date"
          value={startDate}
          onChange={(event) => onStartDateChange(event.target.value)}
        />
      </Field>
      <Field className="space-y-2">
        <Label htmlFor="rewards-end-date">{tPairings("filters.endDate")}</Label>
        <Input
          id="rewards-end-date"
          type="date"
          value={endDate}
          onChange={(event) => onEndDateChange(event.target.value)}
        />
      </Field>
    </div>
  );
}
