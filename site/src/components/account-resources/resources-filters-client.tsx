"use client";

import { usePathname, useRouter, useSearchParams } from "next/navigation";
import { useExtracted } from "next-intl";
import { useEffect, useState, useTransition } from "react";
import { Field, Label } from "@/components/ui/fieldset";
import { Input } from "@/components/ui/input";
import { MAX_RANGE_DAYS, ONE_DAY_MILLIS, parseDateInput, toDateInput } from "@/lib/loot/date";

type ResourcesFiltersClientProps = {
  startDate: string;
  endDate: string;
  minDate: string;
  maxDate: string;
};

function clampEndDate(startDate: string, endDate: string): string {
  const startMillis = parseDateInput(startDate);
  const endMillis = parseDateInput(endDate);

  if (startMillis == null || endMillis == null) {
    return endDate;
  }

  const maxEndMillis = startMillis + (MAX_RANGE_DAYS - 1) * ONE_DAY_MILLIS;
  if (endMillis > maxEndMillis) {
    return toDateInput(maxEndMillis);
  }

  if (endMillis < startMillis) {
    return startDate;
  }

  return endDate;
}

export function ResourcesFiltersClient({
  startDate,
  endDate,
  minDate,
  maxDate,
}: ResourcesFiltersClientProps) {
  const t = useExtracted();
  const router = useRouter();
  const pathname = usePathname();
  const searchParams = useSearchParams();
  const [isPending, startTransition] = useTransition();
  const [draftStartDate, setDraftStartDate] = useState(startDate);
  const [draftEndDate, setDraftEndDate] = useState(endDate);

  useEffect(() => {
    setDraftStartDate(startDate);
    setDraftEndDate(endDate);
  }, [startDate, endDate]);

  const updateUrl = (nextStartDate: string, nextEndDate: string) => {
    if (!pathname) {
      return;
    }

    const params = new URLSearchParams(searchParams.toString());
    params.set("start", nextStartDate);
    params.set("end", nextEndDate);

    const query = params.toString();
    const href = query ? `${pathname}?${query}` : pathname;

    startTransition(() => {
      router.replace(href, { scroll: false });
    });
  };

  const handleStartDateChange = (value: string) => {
    const nextStartDate = value || startDate;
    const nextEndDate = clampEndDate(nextStartDate, draftEndDate || endDate);

    setDraftStartDate(nextStartDate);
    setDraftEndDate(nextEndDate);
    updateUrl(nextStartDate, nextEndDate);
  };

  const handleEndDateChange = (value: string) => {
    const nextEndDate = clampEndDate(draftStartDate || startDate, value || endDate);

    setDraftEndDate(nextEndDate);
    updateUrl(draftStartDate || startDate, nextEndDate);
  };

  return (
    <div className="grid gap-4 md:grid-cols-2" aria-busy={isPending ? "true" : undefined}>
      <Field className="space-y-2">
        <Label htmlFor="resources-start-date">{t("Start date")}</Label>
        <Input
          id="resources-start-date"
          type="date"
          value={draftStartDate}
          min={minDate}
          max={maxDate}
          onChange={(event) => handleStartDateChange(event.target.value)}
        />
      </Field>
      <Field className="space-y-2">
        <Label htmlFor="resources-end-date">{t("End date")}</Label>
        <Input
          id="resources-end-date"
          type="date"
          value={draftEndDate}
          min={minDate}
          max={maxDate}
          onChange={(event) => handleEndDateChange(event.target.value)}
        />
      </Field>
    </div>
  );
}
