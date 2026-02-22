import { getExtracted } from "next-intl/server";
import { LootBreakdownTable } from "@/components/account-loot/loot-breakdown-table";
import { Subheading } from "@/components/ui/heading";
import { Text } from "@/components/ui/text";
import { buildLootRewardRows } from "@/lib/loot/reward-rows";
import type { LootCategoryAggregate } from "@/lib/types/loot";

type LootTableSectionProps = {
  category: LootCategoryAggregate;
  datasetLocale?: string;
};

export async function LootTableSection({ category, datasetLocale }: LootTableSectionProps) {
  const t = await getExtracted();
  const rows = buildLootRewardRows(
    category,
    (type, subType) =>
      t("Unknown reward {type}/{subType}", {
        type: type.toString(),
        subType: subType.toString(),
      }),
    datasetLocale
  );

  return (
    <section className="space-y-4">
      <Subheading>{t("Loot breakdown")}</Subheading>
      {rows.length === 0 ? (
        <Text>{t("No loot in this date range.")}</Text>
      ) : (
        <LootBreakdownTable rows={rows} />
      )}
    </section>
  );
}
