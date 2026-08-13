export function calculateTradePercentage(gained: number, lost: number): number {
  if (gained === lost) {
    return 100;
  }

  return lost > 0 ? (gained / lost) * 100 : 0;
}
