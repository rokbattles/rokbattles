"use client";

import { Popover } from "@base-ui/react/popover";
import { type DateRange, TZDate } from "@daypicker/react";
import { useExtracted } from "next-intl";
import { useState, useSyncExternalStore } from "react";
import { Button } from "@/components/ui/button";
import { Calendar } from "@/components/ui/calendar";
import { MAX_RANGE_DAYS, toDateInput } from "@/lib/loot/date";
import {
  MIN_RESOURCE_DATE,
  type ResourceDateRange,
  validResourceRange,
} from "@/lib/resources/date-range";

type ResourcesRangePickerProps = ResourceDateRange & {
  today: string;
  onApply: (range: ResourceDateRange) => void;
};

function subscribeToViewport(onChange: () => void) {
  const query = window.matchMedia("(min-width: 40rem)");
  query.addEventListener("change", onChange);
  return () => query.removeEventListener("change", onChange);
}

const isDesktop = () => window.matchMedia("(min-width: 40rem)").matches;
const serverIsDesktop = () => false;
const utcDate = (date: string) => new TZDate(`${date}T00:00:00Z`, "UTC");

export function ResourcesRangePicker({ start, end, today, onApply }: ResourcesRangePickerProps) {
  const t = useExtracted();
  const desktop = useSyncExternalStore(subscribeToViewport, isDesktop, serverIsDesktop);
  const [range, setRange] = useState<DateRange | undefined>(() => ({
    from: utcDate(start),
    to: utcDate(end),
  }));

  const value =
    range?.from && range.to
      ? { start: toDateInput(range.from.getTime()), end: toDateInput(range.to.getTime()) }
      : undefined;
  const valid = value !== undefined && validResourceRange(value, today);

  return (
    <>
      <div className="shrink-0 px-4 pt-4 pb-3">
        <Popover.Title className="text-sm font-semibold text-white">
          {t("Choose a date range")}
        </Popover.Title>
        <Popover.Description className="mt-1 max-w-sm text-xs/5 text-zinc-400">
          {t("Select a start and end date, up to {days} days.", {
            days: String(MAX_RANGE_DAYS),
          })}
        </Popover.Description>
      </div>
      <div className="min-h-0 overflow-y-auto px-4 pb-4">
        <Calendar
          mode="range"
          required
          resetOnSelect
          autoFocus
          timeZone="UTC"
          today={utcDate(today)}
          defaultMonth={utcDate(start)}
          startMonth={utcDate(MIN_RESOURCE_DATE)}
          endMonth={utcDate(today)}
          disabled={{ before: utcDate(MIN_RESOURCE_DATE), after: utcDate(today) }}
          numberOfMonths={desktop ? 2 : 1}
          selected={range}
          onSelect={setRange}
        />
      </div>
      <div className="shrink-0 space-y-3 border-t border-white/10 p-4">
        {value && !valid ? (
          <p className="max-w-sm text-xs/5 text-red-400" role="alert">
            {t("Choose a valid range of up to {days} days, ending today or earlier.", {
              days: String(MAX_RANGE_DAYS),
            })}
          </p>
        ) : null}
        <div className="flex justify-end gap-2">
          <Popover.Close render={<Button plain>{t("Cancel")}</Button>} />
          <Button color="blue" disabled={!valid} onClick={() => value && valid && onApply(value)}>
            {t("Apply range")}
          </Button>
        </div>
      </div>
    </>
  );
}
