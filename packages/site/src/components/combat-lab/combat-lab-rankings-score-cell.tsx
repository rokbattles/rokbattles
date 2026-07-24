import { cn } from "cnfast";
import { TableCell } from "@/components/ui/table";
import { scoreFormatter } from "@/lib/combat-lab/format";

type CombatLabRankingsScoreCellProps = {
  score: number;
  className?: string;
};

export function CombatLabRankingsScoreCell({ score, className }: CombatLabRankingsScoreCellProps) {
  return (
    <TableCell className={cn("text-right tabular-nums", className)}>
      {scoreFormatter.format(score)}
    </TableCell>
  );
}
