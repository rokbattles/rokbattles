"use client";

import {
  type ChevronProps,
  type DayButtonProps,
  DayPicker,
  type DayPickerProps,
} from "@daypicker/react";
import { cn } from "cn";
import { ChevronLeft, ChevronRight } from "lucide-react";

import { type ComponentProps, useEffect, useMemo, useRef } from "react";
import { Button } from "./button";

export function Calendar({ className, classNames, components, ...props }: DayPickerProps) {
  const timeZone = props.timeZone ?? "UTC";
  const monthFormatter = useMemo(
    () => new Intl.DateTimeFormat("en-US", { month: "long", year: "numeric", timeZone }),
    [timeZone]
  );
  const shortWeekdayFormatter = useMemo(
    () => new Intl.DateTimeFormat("en-US", { weekday: "short", timeZone }),
    [timeZone]
  );
  const dayFormatter = useMemo(
    () => new Intl.DateTimeFormat("en-US", { day: "numeric", timeZone }),
    [timeZone]
  );
  const fullDateFormatter = useMemo(
    () => new Intl.DateTimeFormat("en-US", { dateStyle: "full", timeZone }),
    [timeZone]
  );
  const weekdayFormatter = useMemo(
    () => new Intl.DateTimeFormat("en-US", { weekday: "long", timeZone }),
    [timeZone]
  );

  return (
    <DayPicker
      timeZone={timeZone}
      weekStartsOn={1}
      fixedWeeks
      navLayout="around"
      className={cn(
        "w-fit [--cell-size:min(2.75rem,calc((100vw-3.5rem)/7))] sm:[--cell-size:2.5rem]",
        className
      )}
      classNames={{
        months: "flex gap-6",
        month: "relative",
        month_caption: "flex h-10 items-center justify-center px-10 mb-2",
        caption_label: "text-sm font-semibold text-white",
        month_grid: "border-collapse",
        weekday: "h-8 w-(--cell-size) text-xs font-normal text-zinc-400",
        day: "size-(--cell-size) p-0 text-center",
        range_start: "rounded-l-lg bg-blue-500/15",
        range_end: "rounded-r-lg bg-blue-500/15",
        range_middle: "bg-blue-500/15",
        outside: "text-zinc-500",
        hidden: "invisible",
        button_previous: "absolute! top-0 left-0",
        button_next: "absolute! top-0 right-0",
        ...classNames,
      }}
      components={{
        DayButton: CalendarDayButton,
        PreviousMonthButton: CalendarNavigationButton,
        NextMonthButton: CalendarNavigationButton,
        Chevron: CalendarChevron,
        ...components,
      }}
      formatters={{
        formatCaption: (date) => monthFormatter.format(date),
        formatWeekdayName: (date) => shortWeekdayFormatter.format(date),
        formatDay: (date) => dayFormatter.format(date),
      }}
      labels={{
        labelPrevious: () => "Previous month",
        labelNext: () => "Next month",
        labelGrid: (date) => monthFormatter.format(date),
        labelDayButton: (date) => fullDateFormatter.format(date),
        labelWeekday: (date) => weekdayFormatter.format(date),
      }}
      {...props}
    />
  );
}

function CalendarDayButton({ day, modifiers, className, color: _color, ...props }: DayButtonProps) {
  const ref = useRef<HTMLButtonElement>(null);

  useEffect(() => {
    if (modifiers.focused) ref.current?.focus();
  }, [modifiers.focused]);

  return (
    <Button
      {...props}
      ref={ref}
      plain
      aria-current={modifiers.today ? "date" : undefined}
      className={cn(
        "size-(--cell-size) items-center! rounded-lg p-0! text-sm! font-normal! tabular-nums [&>span[aria-hidden]]:hidden",
        "focus-visible:z-10 focus-visible:outline-offset-0!",
        modifiers.today && "ring-1 ring-inset ring-white/30",
        modifiers.selected &&
          !modifiers.range_middle &&
          "bg-blue-500! text-white! hover:bg-blue-400!",
        modifiers.outside && !modifiers.selected && "text-zinc-500!",
        modifiers.disabled && "text-zinc-600! opacity-50",
        className
      )}
    >
      {props.children}
    </Button>
  );
}

function CalendarNavigationButton({
  className,
  color: _color,
  ...props
}: ComponentProps<"button">) {
  return (
    <Button
      {...props}
      plain
      disabled={props.disabled || props["aria-disabled"] === true}
      className={cn("size-9 items-center! p-0!", className)}
    >
      {props.children}
    </Button>
  );
}

function CalendarChevron({ orientation }: ChevronProps) {
  const Icon = orientation === "left" ? ChevronLeft : ChevronRight;
  return <Icon data-slot="icon" aria-hidden="true" />;
}
