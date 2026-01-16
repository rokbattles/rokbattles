"use client";

import { useTranslations } from "next-intl";
import { useEffect, useState } from "react";
import { PairingRow } from "@/components/trends/pairing-row";
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
} from "@/components/ui/table";
import { Text } from "@/components/ui/text";
import type { CategoryKey, CategorySnapshot, TrendSnapshot } from "@/lib/types/trends";

const CATEGORY_META: Record<CategoryKey, { titleKey: string }> = {
  field: { titleKey: "categories.field" },
  rally: { titleKey: "categories.rally" },
  garrison: { titleKey: "categories.garrison" },
};

function formatDate(value: string | undefined, unknownLabel: string) {
  if (!value) {
    return unknownLabel;
  }
  const parsed = new Date(value);
  if (Number.isNaN(parsed.getTime())) {
    return value;
  }
  return parsed.toLocaleString();
}

function resolveMinCount(snapshot: TrendSnapshot, category: CategoryKey) {
  const fieldFallback = snapshot.minReportCount ?? 5000;
  const categoryCounts = snapshot.minReportCounts;
  if (categoryCounts) {
    return categoryCounts[category] ?? fieldFallback;
  }
  if (category === "field") {
    return fieldFallback;
  }
  return Math.max(1, Math.floor(fieldFallback / 2));
}

export default function PairingTrendsContent() {
  const t = useTranslations("trends");
  const tCommon = useTranslations("common");
  const [snapshot, setSnapshot] = useState<TrendSnapshot | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [loading, setLoading] = useState(true);

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

  const categoryData = snapshot?.categories;
  const categories: Array<[CategoryKey, CategorySnapshot]> = [];
  if (categoryData) {
    (["field", "rally", "garrison"] as CategoryKey[]).forEach((key) => {
      const category = categoryData[key];
      if (category) {
        categories.push([key, category]);
      }
    });
  }

  if (loading) {
    return (
      <Text className="mt-6 text-sm text-zinc-500" role="status" aria-live="polite">
        {t("states.loading")}
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

  if (!snapshot) {
    return (
      <Text className="mt-6 text-sm text-zinc-500" role="status" aria-live="polite">
        {t("states.empty")}
      </Text>
    );
  }

  return (
    <div className="mt-8 space-y-10">
      <div className="space-y-3">
        <Heading>{t("overview.title")}</Heading>
        <DescriptionList>
          <DescriptionTerm>{t("overview.period")}</DescriptionTerm>
          <DescriptionDetails>
            {snapshot.period?.label ?? t("overview.periodFallback")}
          </DescriptionDetails>
          <DescriptionTerm>{t("overview.minimumReports")}</DescriptionTerm>
          <DescriptionDetails>
            {t("overview.minimumReportCounts", {
              field: resolveMinCount(snapshot, "field"),
              rally: resolveMinCount(snapshot, "rally"),
              garrison: resolveMinCount(snapshot, "garrison"),
            })}
          </DescriptionDetails>
          <DescriptionTerm>{t("overview.generated")}</DescriptionTerm>
          <DescriptionDetails>
            {formatDate(snapshot.generatedAt, tCommon("labels.unknown"))}
          </DescriptionDetails>
        </DescriptionList>
      </div>

      {categories.map(([key, category]) => {
        const meta = CATEGORY_META[key];
        const pairings = category?.pairings ?? [];

        return (
          <section key={key} className="space-y-4">
            <div className="flex flex-wrap items-baseline justify-between gap-3">
              <div>
                <Subheading>{t(meta.titleKey)}</Subheading>
                <Text className="mt-1 text-sm text-zinc-500">
                  {t("overview.categorySummary", {
                    pairings: pairings.length,
                    reports: category.totalReports,
                    min: resolveMinCount(snapshot, key),
                  })}
                </Text>
              </div>
            </div>

            <Table dense className="[--gutter:--spacing(4)] lg:[--gutter:--spacing(6)]">
              <TableHead>
                <TableRow>
                  <TableHeader className="w-12">{t("table.rank")}</TableHeader>
                  <TableHeader>{t("table.pairing")}</TableHeader>
                  <TableHeader>{t("table.topAccessory")}</TableHeader>
                  <TableHeader className="w-32">{t("table.reports")}</TableHeader>
                </TableRow>
              </TableHead>
              <TableBody>
                {pairings.length > 0 ? (
                  pairings.map((pairing, index) => (
                    <PairingRow
                      key={`${pairing.primaryCommanderId}:${pairing.secondaryCommanderId}`}
                      pairing={pairing}
                      categoryKey={key}
                      index={index + 1}
                    />
                  ))
                ) : (
                  <TableRow>
                    <TableCell colSpan={4}>
                      <Text className="text-sm text-zinc-500">{t("states.thresholdEmpty")}</Text>
                    </TableCell>
                  </TableRow>
                )}
              </TableBody>
            </Table>
          </section>
        );
      })}
    </div>
  );
}
