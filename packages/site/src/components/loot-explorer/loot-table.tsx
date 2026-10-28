import { LootIcon } from "@/components/account-loot/loot-icon";
import {
  Table,
  TableBody,
  TableCell,
  TableHead,
  TableHeader,
  TableRow,
} from "@/components/ui/table";
import { getLootName, getLootSprites } from "@/lib/loot-catalog";
import type { LootDrop } from "@/lib/loot-explorer/api";
import { formatNumber, formatPercent, formatQuantity } from "@/lib/loot-explorer/format";

export function LootTable({
  loot,
  noItem,
}: {
  loot: LootDrop[];
  noItem?: { dropRate: number; results: number };
}) {
  const sortedLoot = [...loot].sort((left, right) => right.dropRate - left.dropRate);

  if (sortedLoot.length === 0 && !noItem) {
    return (
      <div className="rounded-lg border px-4 py-6 text-sm border-white/10 text-zinc-400">
        No drops have been observed for this selection.
      </div>
    );
  }

  return (
    <Table dense>
      <TableHead>
        <TableRow>
          <TableHeader>Item</TableHeader>
          <TableHeader className="w-28">Quantity</TableHeader>
          <TableHeader className="w-28">Drop Rate</TableHeader>
          <TableHeader className="w-24">Seen</TableHeader>
        </TableRow>
      </TableHead>
      <TableBody>
        {sortedLoot.map((item) => {
          const name =
            getLootName(item.type, item.subType) ??
            `Item ${item.type.toString()}:${item.subType.toString()}`;
          return (
            <TableRow key={`${item.type}:${item.subType}`}>
              <TableCell>
                <div className="flex items-center gap-3">
                  <LootIcon spriteUrls={getLootSprites(item.type, item.subType)} />
                  <div>
                    <div className="font-medium text-white">{name}</div>
                  </div>
                </div>
              </TableCell>
              <TableCell className="w-28">{formatQuantity(item.quantity)}</TableCell>
              <TableCell className="w-28">{formatPercent(item.dropRate)}</TableCell>
              <TableCell className="w-24">{formatNumber(item.results)}</TableCell>
            </TableRow>
          );
        })}
        {noItem ? (
          <TableRow>
            <TableCell className="text-zinc-400">No item</TableCell>
            <TableCell className="w-28">—</TableCell>
            <TableCell className="w-28">{formatPercent(noItem.dropRate)}</TableCell>
            <TableCell className="w-24">{formatNumber(noItem.results)}</TableCell>
          </TableRow>
        ) : null}
      </TableBody>
    </Table>
  );
}
