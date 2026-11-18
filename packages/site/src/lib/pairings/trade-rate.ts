// Wilson score interval, z = 1.959963984540054 for a two-sided 95% interval.
// https://www.itl.nist.gov/div898/handbook/prc/section2/prc241.htm
// This models battles as independent trials; it does not adjust for opponent or session clustering.
export function positiveTradeRate(positive: number, count: number) {
  if (
    !Number.isFinite(positive) ||
    !Number.isFinite(count) ||
    count <= 0 ||
    positive < 0 ||
    positive > count
  )
    return null;

  const rate = positive / count;
  const zSquared = 1.959963984540054 ** 2;
  const denominator = 1 + zSquared / count;
  const center = (rate + zSquared / (2 * count)) / denominator;
  const halfWidth =
    Math.sqrt((zSquared / count) * (rate * (1 - rate) + zSquared / (4 * count))) / denominator;

  return {
    rate: rate * 100,
    lower: Math.max(0, center - halfWidth) * 100,
    upper: Math.min(1, center + halfWidth) * 100,
  };
}
