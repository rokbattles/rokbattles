"use client";

import { useTranslations } from "next-intl";
import { useEffect, useId, useState } from "react";
import { Button } from "@/components/ui/button";
import {
  DescriptionDetails,
  DescriptionList,
  DescriptionTerm,
} from "@/components/ui/description-list";
import { Heading, Subheading } from "@/components/ui/heading";
import {
  Table,
  TableBody,
  TableCell,
  TableHead,
  TableHeader,
  TableRow,
  TableRowHeader,
} from "@/components/ui/table";
import { Text } from "@/components/ui/text";
import { getCommanderName } from "@/hooks/use-commander-name";
import { getEquipmentName } from "@/hooks/use-equipment-name";
import type { CategoryKey, TrendSnapshot } from "@/lib/types/trends";

const CATEGORY_LABELS: Record<CategoryKey, string> = {
  field: "categories.field",
  rally: "categories.rally",
  garrison: "categories.garrison",
};

export default function PairingAccessoryDetails({
  category,
  primaryId,
  secondaryId,
}: {
  category: string;
  primaryId: number;
  secondaryId: number;
}) {
  const t = useTranslations("trends");
  const tCommon = useTranslations("common");
  const [snapshot, setSnapshot] = useState<TrendSnapshot | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [loading, setLoading] = useState(true);
  const [pairsExpanded, setPairsExpanded] = useState(false);
  const [accessoriesExpanded, setAccessoriesExpanded] = useState(false);
  const pairsId = useId();
  const accessoriesId = useId();
  const primaryName = getCommanderName(primaryId) ?? String(primaryId);
  const secondaryName = getCommanderName(secondaryId) ?? String(secondaryId);

  useEffect(() => {
    let active = true;

    const load = async () => {
      try {
        const response = await fetch("/api/v2/trends/pairing-accessories", {
          cache: "no-store",
        });
        if (!response.ok) {
          throw new Error(t("errors.fetch", { status: response.status }));
        }
        const data = (await response.json()) as TrendSnapshot;
        if (active) {
          setSnapshot(data);
          setError(null);
        }
      } catch (err) {
        if (active) {
          setError(err instanceof Error ? err.message : t("errors.generic"));
        }
      } finally {
        if (active) {
          setLoading(false);
        }
      }
    };

    load();
    return () => {
      active = false;
    };
  }, [t]);

  const categoryKey: CategoryKey | null =
    category === "field" || category === "rally" || category === "garrison" ? category : null;
  const pairing =
    snapshot?.categories && categoryKey
      ? (snapshot.categories[categoryKey]?.pairings.find(
          (entry) =>
            entry.primaryCommanderId === primaryId && entry.secondaryCommanderId === secondaryId
        ) ?? null)
      : null;

  if (loading) {
    return (
      <Text className="mt-6 text-sm text-zinc-500" role="status" aria-live="polite">
        {t("states.detailsLoading")}
      </Text>
    );
  }

  if (error) {
    return (
      <Text
        className="mt-6 text-sm text-rose-600 dark:text-rose-400"
        role="status"
        aria-live="polite"
      >
        {error}
      </Text>
    );
  }

  if (!categoryKey) {
    return (
      <Text className="mt-6 text-sm text-zinc-500" role="status" aria-live="polite">
        {t("states.unknownCategory")}
      </Text>
    );
  }

  if (!pairing) {
    return (
      <Text className="mt-6 text-sm text-zinc-500" role="status" aria-live="polite">
        {t("states.notFound")}
      </Text>
    );
  }

  const visiblePairs = pairsExpanded ? pairing.accessoryPairs : pairing.accessoryPairs.slice(0, 10);
  const hasMorePairs = pairing.accessoryPairs.length > 10;
  const visibleAccessories = accessoriesExpanded
    ? pairing.accessories
    : pairing.accessories.slice(0, 10);
  const hasMoreAccessories = pairing.accessories.length > 10;

  return (
    <div className="mt-8 space-y-8">
      <div className="space-y-3">
        <Heading>{t("details.title")}</Heading>
        <DescriptionList>
          <DescriptionTerm>{tCommon("labels.primary")}</DescriptionTerm>
          <DescriptionDetails>{primaryName}</DescriptionDetails>
          <DescriptionTerm>{tCommon("labels.secondary")}</DescriptionTerm>
          <DescriptionDetails>{secondaryName}</DescriptionDetails>
          <DescriptionTerm>{t("details.category")}</DescriptionTerm>
          <DescriptionDetails>{t(CATEGORY_LABELS[categoryKey])}</DescriptionDetails>
          <DescriptionTerm>{t("overview.period")}</DescriptionTerm>
          <DescriptionDetails>
            {snapshot?.period?.label ?? t("overview.periodFallback")}
          </DescriptionDetails>
          <DescriptionTerm>{t("details.totalReports")}</DescriptionTerm>
          <DescriptionDetails>{pairing.reportCount.toLocaleString()}</DescriptionDetails>
        </DescriptionList>
      </div>

      <section className="space-y-3">
        <Subheading>{t("details.accessoryPairs.title")}</Subheading>
        <Table dense className="[--gutter:--spacing(4)] lg:[--gutter:--spacing(6)]">
          <TableHead>
            <TableRow>
              <TableHeader className="w-12">{t("table.rank")}</TableHeader>
              <TableHeader>{t("details.accessoryPairs.header")}</TableHeader>
              <TableHeader className="w-32">{t("table.reports")}</TableHeader>
            </TableRow>
          </TableHead>
          <TableBody id={pairsId}>
            {pairing.accessoryPairs.length > 0 ? (
              visiblePairs.map((entry, index) => (
                <TableRow key={`${entry.ids[0]}:${entry.ids[1]}`}>
                  <TableRowHeader className="w-12 tabular-nums">{index + 1}</TableRowHeader>
                  <TableCell>
                    {getEquipmentName(entry.ids[0]) ?? tCommon("labels.unknown")}{" "}
                    <span className="text-zinc-600 dark:text-zinc-400">
                      {tCommon("labels.and")}
                    </span>{" "}
                    {getEquipmentName(entry.ids[1]) ?? tCommon("labels.unknown")}
                  </TableCell>
                  <TableCell className="w-32 tabular-nums">
                    {entry.count.toLocaleString()}
                  </TableCell>
                </TableRow>
              ))
            ) : (
              <TableRow>
                <TableCell colSpan={3}>
                  <Text className="text-sm text-zinc-500">{t("accessories.emptyPairs")}</Text>
                </TableCell>
              </TableRow>
            )}
          </TableBody>
        </Table>
        {hasMorePairs ? (
          <Button
            plain
            type="button"
            onClick={() => setPairsExpanded((prev) => !prev)}
            aria-expanded={pairsExpanded}
            aria-controls={pairsId}
            className="text-sm"
          >
            {pairsExpanded ? tCommon("actions.showLess") : tCommon("actions.showMore")}
          </Button>
        ) : null}
      </section>

      <section className="space-y-3">
        <Subheading>{t("details.accessories.title")}</Subheading>
        <Table dense className="[--gutter:--spacing(4)] lg:[--gutter:--spacing(6)]">
          <TableHead>
            <TableRow>
              <TableHeader className="w-12">{t("table.rank")}</TableHeader>
              <TableHeader>{t("details.accessories.header")}</TableHeader>
              <TableHeader className="w-32">{t("table.reports")}</TableHeader>
            </TableRow>
          </TableHead>
          <TableBody id={accessoriesId}>
            {pairing.accessories.length > 0 ? (
              visibleAccessories.map((entry, index) => (
                <TableRow key={entry.id}>
                  <TableRowHeader className="w-12 tabular-nums">{index + 1}</TableRowHeader>
                  <TableCell>{getEquipmentName(entry.id) ?? tCommon("labels.unknown")}</TableCell>
                  <TableCell className="w-32 tabular-nums">
                    {entry.count.toLocaleString()}
                  </TableCell>
                </TableRow>
              ))
            ) : (
              <TableRow>
                <TableCell colSpan={3}>
                  <Text className="text-sm text-zinc-500">{t("accessories.emptyAccessories")}</Text>
                </TableCell>
              </TableRow>
            )}
          </TableBody>
        </Table>
        {hasMoreAccessories ? (
          <Button
            plain
            type="button"
            onClick={() => setAccessoriesExpanded((prev) => !prev)}
            aria-expanded={accessoriesExpanded}
            aria-controls={accessoriesId}
            className="text-sm"
          >
            {accessoriesExpanded ? tCommon("actions.showLess") : tCommon("actions.showMore")}
          </Button>
        ) : null}
      </section>
    </div>
  );
}
