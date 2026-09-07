import { getLocale } from "next-intl/server";
import { CombatLab } from "@/components/combat-lab/combat-lab";
import { getCombatLabCommanderOptions, isCombatLabCommanderId } from "@/lib/combat-lab/commanders";
import { fetchCombatLabPreview } from "@/lib/combat-lab/preview-api";

const defaultPrimaryCommanderId = 509;
const defaultSecondaryCommanderId = 6;

export default async function CombatLabPage({
  searchParams,
}: {
  searchParams: Promise<{ primary?: string; secondary?: string }>;
}) {
  const params = await searchParams;
  const requestedPrimary = combatLabCommanderId(params.primary) ?? defaultPrimaryCommanderId;
  const requestedSecondary = combatLabCommanderId(params.secondary) ?? defaultSecondaryCommanderId;
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
    getCombatLabCommanderOptions(locale),
  ]);
  return <CombatLab commanderOptions={commanderOptions} data={data} />;
}

function combatLabCommanderId(value: string | undefined): number | null {
  const number = Number(value);
  return Number.isSafeInteger(number) && number > 0 && isCombatLabCommanderId(number)
    ? number
    : null;
}
