import { formatNumber } from "@/lib/formatNumber";

type Props = {
  label: string;
  self: number;
  enemy: number;
  locale: string;
};

export async function BattleResultMetric({ label, self, enemy, locale }: Props) {
  const a = Math.max(0, self ?? 0);
  const b = Math.max(0, enemy ?? 0);
  const sum = a + b;
  const lp = sum ? (a / sum) * 100 : 0;
  const rp = 100 - lp;

  return (
    <div className="rounded-lg bg-zinc-800/50 p-3 ring-1 ring-white/5">
      <div className="mb-1 flex items-center justify-between text-xs text-zinc-400">
        <span>{label}</span>
        <span className="text-zinc-500">{formatNumber(sum, locale)}</span>
      </div>
      <div className="flex items-baseline justify-between">
        <div className="text-lg font-semibold text-zinc-100">{formatNumber(self, locale)}</div>
        <div className="text-base text-zinc-100">{formatNumber(enemy, locale)}</div>
      </div>
      <div className="mt-2 grid grid-cols-[1fr_2px_1fr] items-center gap-2">
        <div className="relative h-2 rounded bg-zinc-900">
          {a > 0 && (
            <div
              className="absolute right-0 top-0 h-full rounded-full bg-blue-500"
              style={{ width: `${lp}%` }}
            />
          )}
        </div>
        <div className="h-3 w-[2px] bg-white/10 rounded" />
        <div className="relative h-2 rounded bg-zinc-900">
          {b > 0 && (
            <div
              className="absolute left-0 top-0 h-full rounded-full bg-red-500"
              style={{ width: `${rp}%` }}
            />
          )}
        </div>
      </div>
    </div>
  );
}
