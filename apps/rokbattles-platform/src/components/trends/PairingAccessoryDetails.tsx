"use client";

import { useEffect, useState } from "react";
import type { CategoryKey, TrendSnapshot } from "@/lib/types/trends";
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
import { getCommanderName } from "@/hooks/useCommanderName";
import { getEquipmentName } from "@/hooks/useEquipmentName";

const CATEGORY_LABELS: Record<CategoryKey, string> = {
  field: "Field Reports",
  rally: "Rally Reports",
  garrison: "Garrison Reports",
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
  const [snapshot, setSnapshot] = useState<TrendSnapshot | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [loading, setLoading] = useState(true);
  const [pairsExpanded, setPairsExpanded] = useState(false);
  const [accessoriesExpanded, setAccessoriesExpanded] = useState(false);
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

  const categoryKey: CategoryKey | null =
    category === "field" || category === "rally" || category === "garrison" ? category : null;
  const pairing =
    snapshot?.categories && categoryKey
      ? snapshot.categories[categoryKey]?.pairings.find(
          (entry) =>
            entry.primaryCommanderId === primaryId && entry.secondaryCommanderId === secondaryId
        ) ?? null
      : null;

  if (loading) {
    return <Text className="mt-6 text-sm text-zinc-500">Loading pairing details...</Text>;
  }

  if (error) {
    return <Text className="mt-6 text-sm text-rose-600 dark:text-rose-400">{error}</Text>;
  }

  if (!categoryKey) {
    return <Text className="mt-6 text-sm text-zinc-500">Unknown category.</Text>;
  }

  if (!pairing) {
    return <Text className="mt-6 text-sm text-zinc-500">Pairing not found.</Text>;
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
        <Heading>Trend Details</Heading>
        <DescriptionList>
          <DescriptionTerm>Primary</DescriptionTerm>
          <DescriptionDetails>{primaryName}</DescriptionDetails>
          <DescriptionTerm>Secondary</DescriptionTerm>
          <DescriptionDetails>{secondaryName}</DescriptionDetails>
          <DescriptionTerm>Category</DescriptionTerm>
          <DescriptionDetails>{CATEGORY_LABELS[categoryKey]}</DescriptionDetails>
          <DescriptionTerm>Period</DescriptionTerm>
          <DescriptionDetails>{snapshot?.period?.label ?? "2025 Q4"}</DescriptionDetails>
          <DescriptionTerm>Total reports</DescriptionTerm>
          <DescriptionDetails>{pairing.reportCount.toLocaleString()}</DescriptionDetails>
        </DescriptionList>
      </div>

      <section className="space-y-3">
        <Subheading>Accessory Pairs</Subheading>
        <Table dense className="[--gutter:--spacing(4)] lg:[--gutter:--spacing(6)]">
          <TableHead>
            <TableRow>
              <TableHeader className="text-right">#</TableHeader>
              <TableHeader>Accessory pair</TableHeader>
              <TableHeader className="text-right">Reports</TableHeader>
            </TableRow>
          </TableHead>
          <TableBody>
            {pairing.accessoryPairs.length > 0 ? (
              visiblePairs.map((entry, index) => (
                <TableRow key={`${entry.ids[0]}:${entry.ids[1]}`}>
                  <TableCell className="text-right font-mono text-zinc-500">{index + 1}</TableCell>
                  <TableCell>
                    {getEquipmentName(entry.ids[0]) ?? "Unknown"}{" "}
                    <span className="text-zinc-600 dark:text-zinc-400">and</span>{" "}
                    {getEquipmentName(entry.ids[1]) ?? "Unknown"}
                  </TableCell>
                  <TableCell className="text-right font-mono text-zinc-950 dark:text-white">
                    {entry.count.toLocaleString()}
                  </TableCell>
                </TableRow>
              ))
            ) : (
              <TableRow>
                <TableCell colSpan={3}>
                  <Text className="text-sm text-zinc-500">No accessory pairs recorded.</Text>
                </TableCell>
              </TableRow>
            )}
          </TableBody>
        </Table>
        {hasMorePairs ? (
          <button
            type="button"
            onClick={() => setPairsExpanded((prev) => !prev)}
            className="text-sm font-medium text-blue-600 transition hover:text-blue-500 dark:text-blue-400 dark:hover:text-blue-300"
          >
            {pairsExpanded ? "Show less" : "Show more"}
          </button>
        ) : null}
      </section>

      <section className="space-y-3">
        <Subheading>Individual Accessories</Subheading>
        <Table dense className="[--gutter:--spacing(4)] lg:[--gutter:--spacing(6)]">
          <TableHead>
            <TableRow>
              <TableHeader className="text-right">#</TableHeader>
              <TableHeader>Accessory</TableHeader>
              <TableHeader className="text-right">Reports</TableHeader>
            </TableRow>
          </TableHead>
          <TableBody>
            {pairing.accessories.length > 0 ? (
              visibleAccessories.map((entry, index) => (
                <TableRow key={entry.id}>
                  <TableCell className="text-right font-mono text-zinc-500">{index + 1}</TableCell>
                  <TableCell>{getEquipmentName(entry.id) ?? "Unknown"}</TableCell>
                  <TableCell className="text-right font-mono text-zinc-950 dark:text-white">
                    {entry.count.toLocaleString()}
                  </TableCell>
                </TableRow>
              ))
            ) : (
              <TableRow>
                <TableCell colSpan={3}>
                  <Text className="text-sm text-zinc-500">No accessory data recorded.</Text>
                </TableCell>
              </TableRow>
            )}
          </TableBody>
        </Table>
        {hasMoreAccessories ? (
          <button
            type="button"
            onClick={() => setAccessoriesExpanded((prev) => !prev)}
            className="text-sm font-medium text-blue-600 transition hover:text-blue-500 dark:text-blue-400 dark:hover:text-blue-300"
          >
            {accessoriesExpanded ? "Show less" : "Show more"}
          </button>
        ) : null}
      </section>
    </div>
  );
}
