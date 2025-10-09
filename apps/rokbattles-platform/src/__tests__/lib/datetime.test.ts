import { describe, expect, it } from "vitest";
import { formatDurationShort, formatUtcDateTime } from "@/lib/datetime";

describe("formatDurationShort", () => {
  it("computes multi-unit durations from date objects", () => {
    const start = new Date(Date.UTC(2024, 0, 1, 0, 0, 0));
    const end = new Date(Date.UTC(2024, 0, 2, 2, 3, 4));

    expect(formatDurationShort(start, end)).toBe("1d 2h 3m 4s");
  });

  it("normalizes second-based epoch numbers", () => {
    const start = 1710000000;
    const end = start + 3600;

    expect(formatDurationShort(start, end)).toBe("1h");
  });

  it("supports Mongo $date and $numberLong wrappers", () => {
    const start = { $date: { $numberLong: "1700000000000" } };
    const end = { $date: "2023-11-14T22:13:25.000Z" };

    expect(formatDurationShort(start, end)).toBe("5s");
  });

  it("returns 0s when either value is invalid", () => {
    expect(formatDurationShort(null, new Date())).toBe("0s");
    expect(formatDurationShort("not a date", "still not a date")).toBe("0s");
  });
});

describe("formatUtcDateTime", () => {
  it("formats trimmed ISO strings into UTC display", () => {
    expect(formatUtcDateTime(" 2024-01-31T04:05:00Z ")).toBe("UTC 01/31 04:05");
  });

  it("formats normalized epoch numbers", () => {
    expect(formatUtcDateTime(1_700_000_000)).toBe("UTC 11/14 22:13");
  });

  it("falls back to placeholder when the value is invalid", () => {
    expect(formatUtcDateTime(-100)).toBe("UTC --/-- --:--");
    expect(formatUtcDateTime(undefined)).toBe("UTC --/-- --:--");
  });
});
