export async function resolveLootExplorerSearchParams(
  searchParams: PageProps<"/loot-explorer/barbarians">["searchParams"]
) {
  const params = await searchParams;
  return {
    type: firstParam(params.type),
    levels: numberListParam(params.level),
  };
}

function firstParam(value: string | string[] | undefined): string | undefined {
  if (Array.isArray(value)) {
    return value.find((item) => item.trim().length > 0)?.trim();
  }

  return value?.trim() || undefined;
}

function numberListParam(value: string | string[] | undefined): number[] {
  const rawValues = Array.isArray(value) ? value : value ? [value] : [];
  const parsed = new Set<number>();

  for (const rawValue of rawValues) {
    for (const part of rawValue.split(",")) {
      const parsedValue = Number.parseInt(part.trim(), 10);
      if (Number.isFinite(parsedValue)) {
        parsed.add(parsedValue);
      }
    }
  }

  return Array.from(parsed).sort((left, right) => left - right);
}
