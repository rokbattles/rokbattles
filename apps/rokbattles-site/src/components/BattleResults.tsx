import type { BattleResults } from "@/lib/types/reports";

function StatRow({ label, self, enemy }: { label: string; self?: number; enemy?: number }) {
  const fmt = (n?: number) =>
    typeof n === "number" && !Number.isNaN(n) ? n.toLocaleString() : "—";
  return (
    <div className="grid grid-cols-[1fr_auto_auto] items-baseline gap-4 py-1">
      <div className="text-sm text-zinc-300">{label}</div>
      <div className="w-28 text-sm tabular-nums text-zinc-100 text-right">{fmt(self)}</div>
      <div className="w-28 text-sm tabular-nums text-zinc-100 text-right">{fmt(enemy)}</div>
    </div>
  );
}

export default function BattleResultsView({
  data,
  leftName,
  rightName,
}: {
  data?: BattleResults;
  leftName?: string;
  rightName?: string;
}) {
  if (!data) return null;
  return (
    <section className="rounded-lg border border-white/10 bg-zinc-900/60 p-4">
      <div className="flex items-baseline justify-between">
        <h3 className="text-base font-semibold text-zinc-100">Battle Results</h3>
        <div className="text-[11px] text-zinc-400">
          <span className="font-medium text-zinc-200">{leftName ?? "Self"}</span>
          <span className="mx-1.5 text-zinc-500">/</span>
          <span className="font-medium text-zinc-200">{rightName ?? "Enemy"}</span>
        </div>
      </div>
      <div className="mt-2 space-y-1">
        <StatRow label="Units" self={data.max} enemy={data.enemy_max} />
        <StatRow label="Healing" self={data.healing} enemy={data.enemy_healing} />
        <StatRow label="Dead" self={data.death} enemy={data.enemy_death} />
        <StatRow
          label="Severely wounded"
          self={data.severely_wounded}
          enemy={data.enemy_severely_wounded}
        />
        <StatRow label="Slightly wounded" self={data.wounded} enemy={data.enemy_wounded} />
        <StatRow label="Remaining" self={data.remaining} enemy={data.enemy_remaining} />
        <StatRow label="Watchtower damage" self={data.watchtower} enemy={data.enemy_watchtower} />
        <StatRow label="Kill points" self={data.kill_score} enemy={data.enemy_kill_score} />
      </div>
    </section>
  );
}
