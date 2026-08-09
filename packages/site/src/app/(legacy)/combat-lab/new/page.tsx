import { getLocale } from "next-intl/server";
import { CombatLabPreview } from "@/components/combat-lab/new/combat-lab-preview";
import { fetchCombatLabPreview } from "@/lib/combat-lab/preview-api";

export default async function CombatLabPage({
  searchParams,
}: {
  searchParams: Promise<{ primary?: string; secondary?: string }>;
}) {
  const params = await searchParams;
  const primary = positiveInteger(params.primary) ?? 509;
  const secondary = positiveInteger(params.secondary) ?? 6;
  const locale = await getLocale();
  const data = await fetchCombatLabPreview({ primary, secondary, locale });
  return <CombatLabPreview data={data} />;
}

function positiveInteger(value: string | undefined): number | null {
  const number = Number(value);
  return Number.isSafeInteger(number) && number > 0 ? number : null;
}
