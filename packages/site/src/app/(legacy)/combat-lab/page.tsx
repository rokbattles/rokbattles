import { getLocale } from "next-intl/server";
import { CombatLab } from "@/components/combat-lab/combat-lab";
import { getLegendaryCommanderOptions, isLegendaryCommanderId } from "@/lib/combat-lab/commanders";
import { fetchCombatLabPreview } from "@/lib/combat-lab/preview-api";

const defaultPrimaryCommanderId = 509;
const defaultSecondaryCommanderId = 6;

export default async function CombatLabPage({
  searchParams,
}: {
  searchParams: Promise<{ primary?: string; secondary?: string }>;
}) {
  const params = await searchParams;
  const requestedPrimary = legendaryCommanderId(params.primary) ?? defaultPrimaryCommanderId;
  const requestedSecondary = legendaryCommanderId(params.secondary) ?? defaultSecondaryCommanderId;
  const primary = requestedPrimary;
  const secondary =
    requestedSecondary === primary
      ? primary === defaultSecondaryCommanderId
        ? defaultPrimaryCommanderId
        : defaultSecondaryCommanderId
      : requestedSecondary;
  const locale = await getLocale();
  const [data, commanderOptions] = await Promise.all([
    fetchCombatLabPreview({ primary, secondary, locale }),
    getLegendaryCommanderOptions(locale),
  ]);
  return <CombatLab commanderOptions={commanderOptions} data={data} />;
}

function legendaryCommanderId(value: string | undefined): number | null {
  const number = Number(value);
  return Number.isSafeInteger(number) && number > 0 && isLegendaryCommanderId(number)
    ? number
    : null;
}
