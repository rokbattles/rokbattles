import { getLocale } from "next-intl/server";
import { CombatLab } from "@/components/combat-lab/combat-lab";
import { getCombatLabCommanderOptions, isCombatLabCommanderId } from "@/lib/combat-lab/commanders";
import { fetchCombatLabPreview } from "@/lib/combat-lab/preview-api";
import type { CombatLabSeason } from "@/lib/combat-lab/season";

export async function CombatLabPage({
  searchParams,
  season,
}: {
  season: CombatLabSeason;
  searchParams: Promise<{ primary?: string; secondary?: string }>;
}) {
  const defaultPrimaryCommanderId = season === "presoc" ? 64 : 509;
  const defaultSecondaryCommanderId = 6;
  const params = await searchParams;
  const requestedPrimary =
    combatLabCommanderId(params.primary, season) ?? defaultPrimaryCommanderId;
  const requestedSecondary =
    combatLabCommanderId(params.secondary, season) ?? defaultSecondaryCommanderId;
  const primary = requestedPrimary;
  const secondary =
    requestedSecondary === primary
      ? primary === defaultSecondaryCommanderId
        ? defaultPrimaryCommanderId
        : defaultSecondaryCommanderId
      : requestedSecondary;
  const locale = await getLocale();
  const [data, commanderOptions] = await Promise.all([
    fetchCombatLabPreview({ primary, secondary, locale, season }),
    getCombatLabCommanderOptions(locale, season),
  ]);
  return <CombatLab commanderOptions={commanderOptions} data={data} />;
}

function combatLabCommanderId(value: string | undefined, season: CombatLabSeason): number | null {
  const number = Number(value);
  return Number.isSafeInteger(number) && number > 0 && isCombatLabCommanderId(number, season)
    ? number
    : null;
}
