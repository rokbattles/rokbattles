"use client";

const compactNumberFormatter = new Intl.NumberFormat("en-US", {
  notation: "compact",
  maximumFractionDigits: 1,
});

export function useCompactNumberFormatter(): Intl.NumberFormat {
  return compactNumberFormatter;
}
