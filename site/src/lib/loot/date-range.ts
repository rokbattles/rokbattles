import "server-only";

const ONE_DAY_MILLIS = 24 * 60 * 60 * 1000;

type DateRangeInput = {
  startParam?: string | null;
  endParam?: string | null;
  fallbackYear: number;
  maxRangeDays?: number;
};

export type DateRangeResult = {
  year: number;
  startMillis: number;
  endMillis: number;
  start: string;
  end: string;
};

function parseDateStart(value?: string | null): number | null {
  if (!value) {
    return null;
  }

  const parsed = new Date(`${value}T00:00:00Z`);
  const millis = parsed.getTime();
  return Number.isNaN(millis) ? null : millis;
}

function parseDateEndInclusive(value?: string | null): number | null {
  if (!value) {
    return null;
  }

  const parsed = new Date(`${value}T23:59:59.999Z`);
  const millis = parsed.getTime();
  return Number.isNaN(millis) ? null : millis;
}

function toDateKey(millis: number): string {
  return new Date(millis).toISOString().slice(0, 10);
}

export function resolveDateRange(options: DateRangeInput): DateRangeResult {
  const { startParam, endParam, fallbackYear, maxRangeDays = 366 } = options;
  const parsedStart = parseDateStart(startParam);
  const parsedEndInclusive = parseDateEndInclusive(endParam);

  let startMillis = Date.UTC(fallbackYear, 0, 1, 0, 0, 0, 0);
  let endMillis = Date.UTC(fallbackYear + 1, 0, 1, 0, 0, 0, 0);

  if (parsedStart != null && parsedEndInclusive != null) {
    const endExclusive = parsedEndInclusive + 1;
    if (endExclusive > parsedStart) {
      startMillis = parsedStart;
      endMillis = endExclusive;
    }
  }

  const maxRangeMillis = maxRangeDays * ONE_DAY_MILLIS;
  const cappedEndMillis = Math.min(endMillis, startMillis + maxRangeMillis);
  const finalEndMillis = Math.max(cappedEndMillis, startMillis + ONE_DAY_MILLIS);

  return {
    year: new Date(startMillis).getUTCFullYear(),
    startMillis,
    endMillis: finalEndMillis,
    start: toDateKey(startMillis),
    end: toDateKey(finalEndMillis - 1),
  };
}
