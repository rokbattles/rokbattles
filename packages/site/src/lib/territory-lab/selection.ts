export function updatePlannerSelection(
  current: ReadonlySet<string>,
  itemId: string | null,
  additive: boolean
): Set<string> {
  if (!itemId) return additive ? new Set(current) : new Set();
  if (!additive) return new Set([itemId]);

  const next = new Set(current);
  if (next.has(itemId)) next.delete(itemId);
  else next.add(itemId);
  return next;
}
