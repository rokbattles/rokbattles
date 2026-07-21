import { getLocale } from "next-intl/server";
import { CombatLabContent } from "@/components/combat-lab/combat-lab-content";
import { CombatLabLayout } from "@/components/combat-lab/combat-lab-layout";
import { getLegendaryCommanderOptions } from "@/lib/combat-lab/commanders";

export default async function Page() {
  const locale = await getLocale();
  const commanderOptions = getLegendaryCommanderOptions(locale);

  return (
    <CombatLabLayout active="explore">
      <CombatLabContent commanderOptions={commanderOptions} />
    </CombatLabLayout>
  );
}
