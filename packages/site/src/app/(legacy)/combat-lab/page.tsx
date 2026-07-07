import { getLocale } from "next-intl/server";
import { CombatLabContent } from "@/components/combat-lab/combat-lab-content";
import { Heading } from "@/components/ui/heading";
import { getLegendaryCommanderOptions } from "@/lib/combat-lab/commanders";

export default async function Page() {
  const locale = await getLocale();
  const commanderOptions = getLegendaryCommanderOptions(locale);

  return (
    <div className="space-y-8">
      <div className="max-w-xl space-y-2">
        <Heading>Combat Lab</Heading>
        <p className="text-sm/6 text-zinc-600 dark:text-zinc-400">
          Explore commander pairing performance with DRASTC scoring when available. Data refreshes
          every 8 hours.
        </p>
      </div>
      <CombatLabContent commanderOptions={commanderOptions} />
    </div>
  );
}
