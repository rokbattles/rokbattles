import { Badge } from "@/components/ui/badge";
import { clampScore, scoreFormatter } from "@/lib/combat-lab/format";

type BreakdownScoreBarProps = {
  badge?: string;
  description?: string;
  label: string;
  score: number;
};

export function BreakdownScoreBar({ badge, description, label, score }: BreakdownScoreBarProps) {
  const markerPosition = clampScore(score) * 10;

  return (
    <div className="space-y-2">
      <div className="flex items-baseline justify-between gap-3">
        <div className="flex min-w-0 items-center gap-2">
          {badge ? <Badge>{badge}</Badge> : null}
          <div className="truncate font-semibold text-sm text-zinc-950 dark:text-white">
            {label}
          </div>
        </div>
        <div className="font-semibold text-sm tabular-nums text-zinc-950 dark:text-white">
          {scoreFormatter.format(score)}
        </div>
      </div>
      <div className="relative h-2 rounded-full bg-zinc-950/10 dark:bg-white/10">
        <div
          className="absolute top-1/2 h-4 w-0.5 -translate-y-1/2 rounded-full bg-white shadow ring-1 ring-zinc-900/20 dark:bg-white"
          style={{ left: `calc(${markerPosition}% - 1px)` }}
        />
        <div className="h-full rounded-full bg-blue-600" style={{ width: `${markerPosition}%` }} />
      </div>
      {description ? (
        <p className="text-sm/6 text-zinc-600 dark:text-zinc-400">{description}</p>
      ) : null}
    </div>
  );
}
