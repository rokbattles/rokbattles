"use client";

import { useEffect, useState } from "react";
import { PairingRow } from "@/components/trends/PairingRow";
import {
  DescriptionDetails,
  DescriptionList,
  DescriptionTerm,
} from "@/components/ui/DescriptionList";
import { Heading, Subheading } from "@/components/ui/Heading";
import {
  Table,
  TableBody,
  TableCell,
  TableHead,
  TableHeader,
  TableRow,
} from "@/components/ui/Table";
import { Text } from "@/components/ui/Text";
import type { CategoryKey, CategorySnapshot, TrendSnapshot } from "@/lib/types/trends";

const CATEGORY_META: Record<CategoryKey, { title: string }> = {
  field: {
    title: "Field Reports",
  },
  rally: {
    title: "Rally Reports",
  },
  garrison: {
    title: "Garrison Reports",
  },
};

function formatDate(value?: string) {
  if (!value) {
    return "Unknown";
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
          throw new Error(`Failed to load trends (${response.status})`);
        }
        const data = (await response.json()) as TrendSnapshot;
        if (active) {
          setSnapshot(data);
          setError(null);
        }
      } catch (err) {
        if (active) {
          setError(err instanceof Error ? err.message : "Failed to load trends");
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
  }, []);

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
    return <Text className="mt-6 text-sm text-zinc-500">Loading pairing trends...</Text>;
  }

  if (error) {
    return <Text className="mt-6 text-sm text-rose-600 dark:text-rose-400">{error}</Text>;
  }

  if (!snapshot) {
    return <Text className="mt-6 text-sm text-zinc-500">No trend snapshot found.</Text>;
  }

  return (
    <div className="mt-8 space-y-10">
      <div className="space-y-3">
        <Heading>Trends Overview</Heading>
        <DescriptionList>
          <DescriptionTerm>Period</DescriptionTerm>
          <DescriptionDetails>{snapshot.period?.label ?? "2025 Q4"}</DescriptionDetails>
          <DescriptionTerm>Minimum reports per pairing</DescriptionTerm>
          <DescriptionDetails>
            Field {resolveMinCount(snapshot, "field").toLocaleString()}; Rally{" "}
            {resolveMinCount(snapshot, "rally").toLocaleString()}; Garrison{" "}
            {resolveMinCount(snapshot, "garrison").toLocaleString()}
          </DescriptionDetails>
          <DescriptionTerm>Generated</DescriptionTerm>
          <DescriptionDetails>{formatDate(snapshot.generatedAt)}</DescriptionDetails>
        </DescriptionList>
      </div>

      {categories.map(([key, category]) => {
        const meta = CATEGORY_META[key];
        const pairings = category?.pairings ?? [];

        return (
          <section key={key} className="space-y-4">
            <div className="flex flex-wrap items-baseline justify-between gap-3">
              <div>
                <Subheading>{meta.title}</Subheading>
                <Text className="mt-1 text-sm text-zinc-500">
                  {pairings.length.toLocaleString()} Pairings in{" "}
                  {category.totalReports.toLocaleString()} Reports (min{" "}
                  {resolveMinCount(snapshot, key).toLocaleString()})
                </Text>
              </div>
            </div>

            <Table dense className="[--gutter:--spacing(4)] lg:[--gutter:--spacing(6)]">
              <TableHead>
                <TableRow>
                  <TableHeader className="text-right">#</TableHeader>
                  <TableHeader>Pairing</TableHeader>
                  <TableHeader>Top accessory</TableHeader>
                  <TableHeader className="text-right">Reports</TableHeader>
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
                      <Text className="text-sm text-zinc-500">
                        No pairings met the threshold for this category.
                      </Text>
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
