import { TableCell } from "@/components/ui/table";
import { scoreFormatter } from "@/lib/combat-lab/format";

type CombatLabRankingsScoreCellProps = {
  score: number;
};

export function CombatLabRankingsScoreCell({ score }: CombatLabRankingsScoreCellProps) {
  return <TableCell className="text-right tabular-nums">{scoreFormatter.format(score)}</TableCell>;
}
