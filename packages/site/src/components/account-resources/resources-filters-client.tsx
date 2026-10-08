"use client";

import { cn } from "cn";
import { CalendarDays, ChevronLeft, ChevronRight } from "lucide-react";
import { useRouter, useSearchParams } from "next/navigation";
import { useExtracted, useFormatter } from "next-intl";
import { type ReactElement, useState, useTransition } from "react";
import { Button } from "@/components/ui/button";
import {
  Dialog,
  DialogActions,
  DialogBody,
  DialogDescription,
  DialogTitle,
} from "@/components/ui/dialog";
import { Link } from "@/components/ui/link";
import { MAX_RANGE_DAYS, toDateInput } from "@/lib/loot/date";
import {
  MIN_RESOURCE_DATE,
  type ResourceDateRange,
  resourceCalendarMonth,
  resourcePresetRange,
  validResourceRange,
} from "@/lib/resources/date-range";

type ResourcesFiltersClientProps = {
  startDate: string;
  endDate: string;
  maxDate: string;
};

type ResourceRangeDialogProps = ResourceDateRange & {
  today: string;
  onClose: () => void;
  onApply: (range: ResourceDateRange) => void;
};

type ResourceCalendarProps = {
  month: string;
  range: ResourceDateRange;
  today: string;
  onSelect: (date: string) => void;
};

export function ResourcesFiltersClient({
  startDate,
  endDate,
  maxDate,
}: ResourcesFiltersClientProps): ReactElement {
  const t = useExtracted();
  const intl = useFormatter();
  const router = useRouter();
  const searchParams = useSearchParams();
  const [isPending, startTransition] = useTransition();
  const [open, setOpen] = useState(false);

  const formatDate = (date: string): string =>
    intl.dateTime(new Date(`${date}T00:00:00Z`), {
      month: "short",
      day: "numeric",
      year: "numeric",
      timeZone: "UTC",
    });
  const presets = [
    { id: "last7", label: t("Last 7d") },
    { id: "last30", label: t("Last 30d") },
    { id: "last90", label: t("Last 90d") },
    { id: "ytd", label: t("YTD") },
  ] as const;

  const apply = (range: ResourceDateRange): void => {
    const params = new URLSearchParams(window.location.search);
    params.set("start", range.start);
    params.set("end", range.end);

    startTransition(() =>
      router.replace(`${window.location.pathname}?${params}`, { scroll: false })
    );

    setOpen(false);
  };

  return (
    <div
      className="flex flex-wrap items-center justify-between gap-3 border-b border-zinc-950/10 dark:border-white/10"
      aria-busy={isPending}
    >
      <nav className="flex gap-5" aria-label={t("Resource date ranges")}>
        {presets.map(({ id, label }) => {
          const range = resourcePresetRange(id, maxDate);
          const active = range.start === startDate && range.end === endDate;
          const params = new URLSearchParams(searchParams.toString());
          params.set("start", range.start);
          params.set("end", range.end);

          return (
            <Link
              key={id}
              href={`?${params}`}
              replace
              scroll={false}
              prefetch={false}
              aria-current={active ? "page" : undefined}
              className={`-mb-px whitespace-nowrap border-b-2 py-3 text-sm font-medium transition-colors focus-visible:outline-2 focus-visible:outline-offset-4 focus-visible:outline-blue-500 ${
                active
                  ? "border-zinc-950 text-zinc-950 dark:border-white dark:text-white"
                  : "border-transparent text-zinc-500 hover:border-zinc-300 hover:text-zinc-950 dark:text-zinc-400 dark:hover:border-zinc-600 dark:hover:text-white"
              }`}
            >
              {label}
            </Link>
          );
        })}
      </nav>
      <Button
        outline
        onClick={() => setOpen(true)}
        className="min-w-0 max-w-full text-xs sm:text-sm"
      >
        <CalendarDays data-slot="icon" />
        <span className="truncate">
          {formatDate(startDate)} – {formatDate(endDate)}
        </span>
      </Button>
      {open ? (
        <ResourceRangeDialog
          start={startDate}
          end={endDate}
          today={maxDate}
          onClose={() => setOpen(false)}
          onApply={apply}
        />
      ) : null}
    </div>
  );
}

function ResourceRangeDialog({
  start,
  end,
  today,
  onClose,
  onApply,
}: ResourceRangeDialogProps): ReactElement {
  const t = useExtracted();
  const intl = useFormatter();
  const [range, setRange] = useState({ start, end });
  const [month, setMonth] = useState(`${start.slice(0, 7)}-01`);
  const [selectingEnd, setSelectingEnd] = useState(false);

  const valid = validResourceRange(range, today);
  const previousMonth = resourceCalendarMonth(month, -1);
  const nextMonth = resourceCalendarMonth(month, 1);

  const chooseDate = (date: string): void => {
    if (!selectingEnd) {
      setRange({ start: date, end: date });
      setSelectingEnd(true);
    } else {
      setRange({
        start: date < range.start ? date : range.start,
        end: date < range.start ? range.start : date,
      });
      setSelectingEnd(false);
    }
  };

  return (
    <Dialog open onClose={onClose} size="2xl">
      <DialogTitle>{t("Choose a date range")}</DialogTitle>
      <DialogDescription>
        {t("Select a start and end day. All dates use UTC; up to {days} days.", {
          days: String(MAX_RANGE_DAYS),
        })}
      </DialogDescription>
      <DialogBody>
        <div className="mb-3 flex items-center justify-between">
          <Button
            plain
            aria-label={t("Previous month")}
            disabled={previousMonth < MIN_RESOURCE_DATE}
            onClick={() => setMonth(previousMonth)}
          >
            <ChevronLeft data-slot="icon" />
          </Button>
          <span className="text-sm text-zinc-500" aria-live="polite">
            {selectingEnd ? t("Select the end day") : t("Select the start day")}
          </span>
          <Button
            plain
            aria-label={t("Next month")}
            disabled={nextMonth > today}
            onClick={() => setMonth(nextMonth)}
          >
            <ChevronRight data-slot="icon" />
          </Button>
        </div>
        <div className="grid gap-6 sm:grid-cols-2">
          <ResourceCalendar month={month} range={range} today={today} onSelect={chooseDate} />
          <div className="hidden sm:block">
            <ResourceCalendar month={nextMonth} range={range} today={today} onSelect={chooseDate} />
          </div>
        </div>
        <div
          className="mt-4 text-center text-sm tabular-nums text-zinc-600 dark:text-zinc-400"
          aria-live="polite"
        >
          {intl.dateTime(new Date(`${range.start}T00:00:00Z`), {
            dateStyle: "medium",
            timeZone: "UTC",
          })}
          {" – "}
          {intl.dateTime(new Date(`${range.end}T00:00:00Z`), {
            dateStyle: "medium",
            timeZone: "UTC",
          })}
        </div>
        {!valid ? (
          <p className="mt-3 text-sm text-red-600 dark:text-red-400" role="alert">
            {t("Choose a valid range of up to {days} days, ending today or earlier.", {
              days: String(MAX_RANGE_DAYS),
            })}
          </p>
        ) : null}
      </DialogBody>
      <DialogActions>
        <Button plain onClick={onClose}>
          {t("Cancel")}
        </Button>
        <Button disabled={!valid} onClick={() => onApply(range)}>
          {t("Apply range")}
        </Button>
      </DialogActions>
    </Dialog>
  );
}

function ResourceCalendar({ month, range, today, onSelect }: ResourceCalendarProps): ReactElement {
  const intl = useFormatter();
  const first = new Date(`${month}T00:00:00Z`);
  const year = first.getUTCFullYear();
  const monthIndex = first.getUTCMonth();
  const offset = (first.getUTCDay() + 6) % 7;
  const days = new Date(Date.UTC(year, monthIndex + 1, 0)).getUTCDate();
  const weekdays = Array.from({ length: 7 }, (_, index) => new Date(Date.UTC(2025, 0, 6 + index)));

  return (
    <div>
      <p className="mb-3 text-center text-sm font-semibold text-zinc-950 dark:text-white">
        {intl.dateTime(first, { month: "long", year: "numeric", timeZone: "UTC" })}
      </p>
      <div className="grid grid-cols-7 text-center text-xs text-zinc-500" aria-hidden="true">
        {weekdays.map((date) => (
          <span key={date.toISOString()} className="py-2">
            {intl.dateTime(date, { weekday: "narrow", timeZone: "UTC" })}
          </span>
        ))}
      </div>
      <div className="grid grid-cols-7 grid-rows-[repeat(6,2.5rem)] gap-y-1">
        {Array.from({ length: days }, (_, index) => {
          const date = toDateInput(Date.UTC(year, monthIndex, index + 1));
          const selected = date >= range.start && date <= range.end;
          const endpoint = date === range.start || date === range.end;
          let dayClasses =
            "text-zinc-700 hover:bg-zinc-100 dark:text-zinc-300 dark:hover:bg-zinc-800";

          if (endpoint) {
            dayClasses = "bg-zinc-900 font-semibold text-white dark:bg-white dark:text-zinc-900";
          } else if (selected) {
            dayClasses = "bg-zinc-100 text-zinc-950 dark:bg-zinc-700 dark:text-white";
          }

          return (
            <button
              type="button"
              key={date}
              onClick={() => onSelect(date)}
              disabled={date < MIN_RESOURCE_DATE || date > today}
              aria-pressed={selected}
              aria-label={intl.dateTime(new Date(`${date}T00:00:00Z`), {
                dateStyle: "full",
                timeZone: "UTC",
              })}
              aria-current={date === today ? "date" : undefined}
              style={index === 0 ? { gridColumnStart: offset + 1 } : undefined}
              className={cn(
                "h-10 rounded-lg text-sm tabular-nums focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-blue-500 disabled:opacity-25",
                dayClasses
              )}
            >
              {index + 1}
            </button>
          );
        })}
      </div>
    </div>
  );
}
