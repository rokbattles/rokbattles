import type { Locale } from "next-intl";
import { resolveNames } from "@/actions/datasets";
import { BattleCommanderSummary } from "@/components/battle/BattleCommanderSummary";
import { BattleParticipant } from "@/components/battle/BattleParticipant";
import { BattleResult } from "@/components/battle/BattleResult";
import { Heading } from "@/components/ui/heading";
import { routing } from "@/i18n/routing";
import type { SingleReportItem } from "@/lib/types/reports";

type BattleReportProps = {
  item?: SingleReportItem;
  locale?: Locale;
};

export async function BattleReport({ item, locale = routing.defaultLocale }: BattleReportProps) {
  if (!item) {
    return (
      <div className="max-w-6xl mx-auto">
        <Heading>Report</Heading>
        <p className="mt-2 text-sm text-zinc-400">No item selected.</p>
      </div>
    );
  }

  const self = item.report?.self;
  const enemy = item.report?.enemy;

  const ids: string[] = [];
  if (self?.primary_commander?.id) ids.push(String(self.primary_commander.id));
  if (self?.secondary_commander?.id) ids.push(String(self.secondary_commander.id));
  if (enemy?.primary_commander?.id) ids.push(String(enemy.primary_commander.id));
  if (enemy?.secondary_commander?.id) ids.push(String(enemy.secondary_commander.id));

  const nameMap = ids.length > 0 ? await resolveNames("commanders", ids, locale) : {};

  return (
    <div className="max-w-6xl mx-auto space-y-4">
      <BattleResult data={item.report.battle_results} self={self} enemy={enemy} locale={locale} />
      <div className="grid gap-4 md:grid-cols-2">
        <section className="rounded-lg bg-zinc-800/50 p-3 ring-1 ring-white/5">
          <div className="space-y-0.5">
            <BattleCommanderSummary info={self?.primary_commander} names={nameMap} />
            <BattleCommanderSummary info={self?.secondary_commander} names={nameMap} />
          </div>
          <div className="mt-3">
            <BattleParticipant participant={self} locale={locale} />
          </div>
        </section>
        <section className="rounded-lg bg-zinc-800/50 p-3 ring-1 ring-white/5">
          <div className="space-y-0.5">
            <BattleCommanderSummary info={enemy?.primary_commander} names={nameMap} />
            <BattleCommanderSummary info={enemy?.secondary_commander} names={nameMap} />
          </div>
          <div className="mt-3">
            <BattleParticipant participant={enemy} locale={locale} />
          </div>
        </section>
      </div>
    </div>
  );
}
