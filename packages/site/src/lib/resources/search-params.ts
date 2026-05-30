export type ResourcesSearchParams = Record<string, string | string[] | undefined>;

export type ParsedResourcesSearchParams = {
  startParam: string | null;
  endParam: string | null;
};

function firstValue(value: string | string[] | undefined): string | null {
  if (Array.isArray(value)) {
    return value[0] ?? null;
  }

  return value ?? null;
}

export function parseResourcesSearchParams(
  searchParams: ResourcesSearchParams
): ParsedResourcesSearchParams {
  const startParam = firstValue(searchParams.start);
  const endParam = firstValue(searchParams.end);

  return {
    startParam,
    endParam,
  };
}
