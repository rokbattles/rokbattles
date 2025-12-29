import ParticipantCell from "@/components/reports/ParticipantCell";
import { AccessoryPairLabel } from "@/components/trends/AccessoryPairLabel";
import {TableCell, TableRow, TableRowHeader} from "@/components/ui/Table";
import type { CategoryKey, PairingSnapshot } from "@/lib/types/trends";

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
      <TableRowHeader className="w-12 tabular-nums">{index}</TableRowHeader>
      <TableCell>
        <ParticipantCell
          primaryId={pairing.primaryCommanderId}
          secondaryId={pairing.secondaryCommanderId}
        />
      </TableCell>
      <TableCell>
        <AccessoryPairLabel pair={topPair} />
      </TableCell>
      <TableCell className="w-32 tabular-nums">{pairing.reportCount.toLocaleString()}</TableCell>
    </TableRow>
  );
}
