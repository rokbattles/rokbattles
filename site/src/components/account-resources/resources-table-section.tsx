"use client";

import { useExtracted } from "next-intl";
import { ResourcesBreakdownTable } from "@/components/account-resources/resources-breakdown-table";
import { Subheading } from "@/components/ui/heading";
import { Text } from "@/components/ui/text";
import { buildResourceBreakdownRows } from "@/lib/resources/rows";
import type { ResourceTotals, ResourceTotalsByType } from "@/lib/types/resources";

type ResourcesTableSectionProps = {
  crystalsGain: ResourceTotals;
  resources: ResourceTotalsByType[];
  datasetLocale?: string;
};

export function ResourcesTableSection({
  crystalsGain,
  resources,
  datasetLocale,
}: ResourcesTableSectionProps) {
  const t = useExtracted();

  const rows = buildResourceBreakdownRows(crystalsGain, resources, datasetLocale);
  const hasAnyTotal = rows.some((row) => row.total > 0);

  return (
    <section className="space-y-4">
      <Subheading>{t("Resource breakdown")}</Subheading>
      {hasAnyTotal ? (
        <ResourcesBreakdownTable rows={rows} />
      ) : (
        <Text>{t("No resources in this date range.")}</Text>
      )}
    </section>
  );
}
