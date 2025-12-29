import ParticipantCell from "@/components/reports/ParticipantCell";
import { AccessoryPairLabel } from "@/components/trends/AccessoryPairLabel";
import type { CategoryKey, PairingSnapshot } from "@/lib/types/trends";
import { TableCell, TableRow } from "@/components/ui/Table";

export function PairingRow({
  pairing,
  categoryKey,
  index,
}: {
  pairing: PairingSnapshot;
  categoryKey: CategoryKey;
  index: number;
}) {
  const topPair = pairing.accessoryPairs[0];
  const pairingLink = `/trends/pairings/${categoryKey}/${pairing.primaryCommanderId}/${pairing.secondaryCommanderId}`;

  return (
    <TableRow href={pairingLink}>
      <TableCell className="text-right font-mono text-zinc-500">{index}</TableCell>
      <TableCell>
        <ParticipantCell
          primaryId={pairing.primaryCommanderId}
          secondaryId={pairing.secondaryCommanderId}
        />
      </TableCell>
      <TableCell>
        <AccessoryPairLabel pair={topPair} />
      </TableCell>
      <TableCell className="text-right font-mono text-zinc-950 dark:text-white">
        {pairing.reportCount.toLocaleString()}
      </TableCell>
    </TableRow>
  );
}
