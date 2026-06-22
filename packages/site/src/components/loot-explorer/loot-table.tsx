import { useExtracted } from "next-intl";
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

export function LootTable({ loot, locale }: { loot: LootDrop[]; locale?: string }) {
  const t = useExtracted();
  const sortedLoot = [...loot].sort((left, right) => right.dropRate - left.dropRate);

  if (sortedLoot.length === 0) {
    return (
      <div className="rounded-lg border border-zinc-950/10 px-4 py-6 text-sm text-zinc-500 dark:border-white/10 dark:text-zinc-400">
        {t("No drops have been observed for this selection.")}
      </div>
    );
  }

  return (
    <Table dense>
      <TableHead>
        <TableRow>
          <TableHeader>{t("Item")}</TableHeader>
          <TableHeader className="w-28">{t("Quantity")}</TableHeader>
          <TableHeader className="w-28">{t("Drop Rate")}</TableHeader>
          <TableHeader className="w-24">{t("Seen")}</TableHeader>
        </TableRow>
      </TableHead>
      <TableBody>
        {sortedLoot.map((item) => {
          const name =
            getLootName(item.type, item.subType, locale) ??
            t("Item {type}:{subType}", {
              subType: item.subType.toString(),
              type: item.type.toString(),
            });
          return (
            <TableRow key={`${item.type}:${item.subType}`}>
              <TableCell>
                <div className="flex items-center gap-3">
                  <LootIcon spriteUrls={getLootSprites(item.type, item.subType)} />
                  <div>
                    <div className="font-medium text-zinc-950 dark:text-white">{name}</div>
                  </div>
                </div>
              </TableCell>
              <TableCell className="w-28">{formatQuantity(item.quantity)}</TableCell>
              <TableCell className="w-28">{formatPercent(item.dropRate)}</TableCell>
              <TableCell className="w-24">{formatNumber(item.results)}</TableCell>
            </TableRow>
          );
        })}
      </TableBody>
    </Table>
  );
}
