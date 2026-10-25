"use client";

import { Popover } from "@base-ui/react/popover";
import { CalendarDays } from "lucide-react";
import dynamic from "next/dynamic";
import { useRouter, useSearchParams } from "next/navigation";
import { useExtracted, useFormatter } from "next-intl";
import { type ReactElement, useState, useTransition } from "react";
import { Button } from "@/components/ui/button";
import { Link } from "@/components/ui/link";
import { type ResourceDateRange, resourcePresetRange } from "@/lib/resources/date-range";

const ResourcesRangePicker = dynamic(
  () => import("./resources-range-picker").then((module) => module.ResourcesRangePicker),
  { loading: ResourcesRangePickerLoading }
);

function ResourcesRangePickerLoading() {
  const t = useExtracted();
  return (
    <div className="space-y-3 p-4">
      <Popover.Title className="text-sm font-semibold">{t("Choose a date range")}</Popover.Title>
      <p className="text-sm text-zinc-400" role="status">
        {t("Loading calendar…")}
      </p>
      <Popover.Close render={<Button plain>{t("Cancel")}</Button>} />
    </div>
  );
}

type ResourcesFiltersClientProps = {
  startDate: string;
  endDate: string;
  maxDate: string;
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
      className="flex flex-wrap items-center justify-between gap-3 border-b border-white/10"
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
                  ? "border-white text-white"
                  : "border-transparent text-zinc-400 hover:border-zinc-600 hover:text-white"
              }`}
            >
              {label}
            </Link>
          );
        })}
      </nav>
      <Popover.Root open={open} onOpenChange={setOpen}>
        <Popover.Trigger
          render={
            <Button outline className="min-w-0 max-w-full text-xs sm:text-sm">
              <CalendarDays data-slot="icon" />
              <span className="truncate">
                {formatDate(startDate)} – {formatDate(endDate)}
              </span>
            </Button>
          }
          aria-label={`${t("Choose a date range")}: ${formatDate(startDate)} – ${formatDate(endDate)}`}
        />
        <Popover.Portal>
          <Popover.Positioner
            side="bottom"
            align="end"
            sideOffset={8}
            collisionPadding={12}
            className="z-50"
          >
            <Popover.Popup
              initialFocus={false}
              className="flex w-min max-h-(--available-height) max-w-(--available-width) flex-col overflow-y-auto rounded-xl bg-zinc-900 shadow-xl ring-1 ring-white/15 focus:outline-hidden"
            >
              {open ? (
                <ResourcesRangePicker
                  key={`${startDate}:${endDate}`}
                  start={startDate}
                  end={endDate}
                  today={maxDate}
                  onApply={apply}
                />
              ) : null}
            </Popover.Popup>
          </Popover.Positioner>
        </Popover.Portal>
      </Popover.Root>
    </div>
  );
}
