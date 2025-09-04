import { BattleResultHeader } from "@/components/battle/BattleResultHeader";
import { BattleResultMetric } from "@/components/battle/BattleResultMetric";
import type { BattleResults, ParticipantInfo } from "@/lib/types/reports";

type Props = {
  data: BattleResults;
  self: ParticipantInfo;
  enemy: ParticipantInfo;
  locale: string;
};

export async function BattleResult({ data, self, enemy, locale }: Props) {
  return (
    <section>
      <BattleResultHeader self={self} enemy={enemy} />
      <div className="grid grid-cols-1 md:grid-cols-2 gap-3">
        <BattleResultMetric label="Units" self={data.max} enemy={data.enemy_max} locale={locale} />
        <BattleResultMetric
          label="Remaining"
          self={data.remaining}
          enemy={data.enemy_remaining}
          locale={locale}
        />
        <BattleResultMetric
          label="Healing"
          self={data.healing}
          enemy={data.enemy_healing}
          locale={locale}
        />
        <BattleResultMetric
          label="Dead"
          self={data.death}
          enemy={data.enemy_death}
          locale={locale}
        />
        <BattleResultMetric
          label="Severely Wounded"
          self={data.severely_wounded}
          enemy={data.enemy_severely_wounded}
          locale={locale}
        />
        <BattleResultMetric
          label="Slightly Wounded"
          self={data.wounded}
          enemy={data.enemy_wounded}
          locale={locale}
        />
        <BattleResultMetric
          label="Watchtower Damage"
          self={data.watchtower}
          enemy={data.enemy_watchtower}
          locale={locale}
        />
        <BattleResultMetric
          label="Kill Points"
          self={data.kill_score}
          enemy={data.enemy_kill_score}
          locale={locale}
        />
      </div>
    </section>
  );
}
