export function formatNumber(num: number, locale: string = "en") {
  return typeof num !== "number" || Number.isNaN(num) ? "\u2014" : num.toLocaleString(locale);
}
