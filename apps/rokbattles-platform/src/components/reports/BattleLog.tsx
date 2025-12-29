"use client";
import { useBattleLog } from "@/hooks/useBattleLog";
import { cn } from "@/lib/cn";
import { Subheading } from "../ui/Heading";
import { Text } from "../ui/Text";

type BattleLogProps = {
  governorId: number;
  year?: number;
};

type DayCell = {
  date: Date;
  key: string;
  battleCount: number;
  npcCount: number;
  inRange: boolean;
};

type WeekColumn = {
  id: string;
  days: DayCell[];
};

function buildSkeletonWeeks(columns = 55) {
  return Array.from({ length: columns }, (_, index) => ({
    id: `placeholder-${index}`,
    days: Array.from({ length: 7 }, (__, dayIndex) => ({
      key: `placeholder-${index}-${dayIndex}`,
      date: new Date(),
      battleCount: 0,
      npcCount: 0,
      inRange: false,
    })),
  }));
}

const dayLabel = new Intl.DateTimeFormat("en-US", {
  timeZone: "UTC",
  month: "short",
  day: "numeric",
  year: "numeric",
});

function parseUtcDateString(value: string) {
  return new Date(`${value}T00:00:00.000Z`);
}

function formatDayKey(date: Date) {
  const year = date.getUTCFullYear();
  const month = date.getUTCMonth() + 1;
  const day = date.getUTCDate();
  return `${year}-${String(month).padStart(2, "0")}-${String(day).padStart(2, "0")}`;
}

function addDays(date: Date, days: number) {
  const next = new Date(date);
  next.setUTCDate(next.getUTCDate() + days);
  return next;
}

function startOfWeekSunday(date: Date) {
  const copy = new Date(Date.UTC(date.getUTCFullYear(), date.getUTCMonth(), date.getUTCDate()));
  const weekday = copy.getUTCDay();
  copy.setUTCDate(copy.getUTCDate() - weekday);
  return copy;
}

function endOfWeekSaturday(date: Date) {
  const copy = new Date(Date.UTC(date.getUTCFullYear(), date.getUTCMonth(), date.getUTCDate()));
  const weekday = copy.getUTCDay();
  copy.setUTCDate(copy.getUTCDate() + (6 - weekday));
  return copy;
}

function buildWeeks(startDate: string, endDate: string, dayMap: Map<string, DayCell>) {
  const rangeStart = parseUtcDateString(startDate);
  const rangeEnd = parseUtcDateString(endDate);
  const gridStart = startOfWeekSunday(rangeStart);
  const gridEnd = endOfWeekSaturday(rangeEnd);

  const weeks: WeekColumn[] = [];
  let cursor = new Date(gridStart);
  let weekIndex = 0;

  while (cursor <= gridEnd) {
    const days: DayCell[] = [];

    for (let i = 0; i < 7; i += 1) {
      const key = formatDayKey(cursor);
      const existing = dayMap.get(key);
      days.push(
        existing ?? {
          key,
          date: new Date(cursor),
          battleCount: 0,
          npcCount: 0,
          inRange: cursor >= rangeStart && cursor <= rangeEnd,
        }
      );
      cursor = addDays(cursor, 1);
    }

    weeks.push({ id: `week-${weekIndex}`, days });
    weekIndex += 1;
  }

  return weeks;
}

function getCellVisuals(day: DayCell) {
  const red500 = "239, 68, 68";
  const yellow500 = "234, 179, 8";

  if (!day.inRange) {
    return {
      className: "border border-transparent bg-transparent",
      style: undefined,
    };
  }

  const total = day.battleCount + day.npcCount;
  if (total === 0) {
    return {
      className: "border border-zinc-200/70 bg-zinc-100/80 dark:border-white/15 dark:bg-white/5",
      style: undefined,
    };
  }

  const strength = Math.min(1, Math.log1p(total) / Math.log(10));
  const battleColor = `rgba(${red500}, ${0.35 + strength * 0.45})`;
  const npcColor = `rgba(${yellow500}, ${0.35 + strength * 0.4})`;

  if (day.npcCount > 0 && day.battleCount > 0) {
    return {
      className: "border border-zinc-200/70 shadow-sm shadow-zinc-900/5 dark:border-white/15",
      style: {
        background: `linear-gradient(135deg, ${npcColor} 0%, ${npcColor} 49.5%, ${battleColor} 49.5%, ${battleColor} 100%)`,
        backgroundClip: "padding-box",
      },
    };
  }

  return {
    className: "border border-white/80 shadow-sm shadow-zinc-900/5 dark:border-white/10",
    style: { backgroundColor: day.npcCount > 0 ? npcColor : battleColor },
  };
}

export function BattleLog({ governorId, year = 2025 }: BattleLogProps) {
  const { data, error, loading } = useBattleLog(governorId, year);
  const interactive = Boolean(data);
  const displayYear = year;

  const dayMap = new Map<string, DayCell>();
  if (data) {
    data.days.forEach((day) => {
      dayMap.set(day.date, {
        key: day.date,
        date: parseUtcDateString(day.date),
        battleCount: day.battleCount,
        npcCount: day.npcCount,
        inRange: true,
      });
    });
  }

  const weeks = data ? buildWeeks(data.startDate, data.endDate, dayMap) : buildSkeletonWeeks();

  const totals = data
    ? data.days.reduce(
        (acc, day) => {
          acc.npc += day.npcCount;
          acc.battle += day.battleCount;
          acc.combined += day.npcCount + day.battleCount;
          return acc;
        },
        { npc: 0, battle: 0, combined: 0 }
      )
    : { npc: 0, battle: 0, combined: 0 };

  return (
    <section className="mt-6 space-y-3">
      <div className="flex flex-wrap items-baseline justify-between gap-3">
        <div className="space-y-1">
          <Subheading>Battle Log</Subheading>
          <Text className="text-sm/6 sm:text-xs/6">
            {loading
              ? `Loading your battle log for ${displayYear}...`
              : `All reports for ${displayYear}. Hover to see counts.`}
          </Text>
        </div>
      </div>
      <div className="overflow-x-auto pb-1">
        <div className="flex items-start gap-3">
          <div className="hidden grid grid-rows-7 gap-[0.2rem] pt-[0.15rem] text-[11px] font-medium text-zinc-500 sm:grid sm:pt-[0.25rem] sm:pr-1 dark:text-zinc-400">
            <span className="row-start-2">Mon</span>
            <span className="row-start-4">Wed</span>
            <span className="row-start-6">Fri</span>
          </div>
          <div className="grid auto-cols-[0.8rem] grid-flow-col gap-[0.2rem] sm:auto-cols-[0.85rem] sm:gap-[0.25rem] md:auto-cols-[0.9rem] md:gap-[0.3rem] lg:auto-cols-[0.95rem]">
            {weeks.map((week) => (
              <div
                key={week.id}
                className="grid grid-rows-7 gap-[0.2rem] sm:gap-[0.25rem] md:gap-[0.3rem]"
              >
                {week.days.map((day, idx) => {
                  const { className, style } = getCellVisuals(day);
                  const label = day.inRange
                    ? `${dayLabel.format(day.date)}: ${day.battleCount} battle report${day.battleCount === 1 ? "" : "s"}, ${day.npcCount} NPC report${day.npcCount === 1 ? "" : "s"}`
                    : "Outside selected range";

                  return (
                    <button
                      key={`${week.id}-${idx}`}
                      type="button"
                      title={interactive && day.inRange ? label : undefined}
                      aria-label={interactive && day.inRange ? label : undefined}
                      tabIndex={interactive && day.inRange ? 0 : -1}
                      className={cn(
                        "relative inline-flex aspect-square h-3 w-3 items-center justify-center border transition duration-150 ease-out focus-visible:outline focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-blue-500 sm:h-3.5 sm:w-3.5 lg:h-4 lg:w-4",
                        className
                      )}
                      style={style}
                    />
                  );
                })}
              </div>
            ))}
          </div>
        </div>
      </div>
      <div className="flex flex-wrap gap-3 text-sm text-zinc-600 sm:text-xs dark:text-zinc-300">
        <span className="font-semibold text-zinc-900 dark:text-white">
          {totals.combined.toLocaleString()} Total reports
        </span>
        <span className="inline-flex items-center gap-2">
          <span className="inline-block size-2.5 rounded-xs bg-red-500" />
          {totals.battle.toLocaleString()} Battles
        </span>
        <span className="inline-flex items-center gap-2">
          <span className="inline-block size-2.5 rounded-xs bg-yellow-500" />
          {totals.npc.toLocaleString()} NPC
        </span>
        {error ? (
          <span className="text-amber-600 dark:text-amber-300">Failed to load battle log</span>
        ) : null}
      </div>
    </section>
  );
}
