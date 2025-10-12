"use client";

import { useContext, useMemo } from "react";
import { GovernorContext } from "@/components/context/GovernorContext";
import { Heading } from "@/components/ui/Heading";
import {
  Table,
  TableBody,
  TableCell,
  TableHead,
  TableHeader,
  TableRow,
} from "@/components/ui/Table";
import { Text } from "@/components/ui/Text";
import { useCommanderName } from "@/hooks/useCommanderName";
import { useCurrentUser } from "@/hooks/useCurrentUser";
import { useMyPairings } from "@/hooks/useMyPairings";

function CommanderPairCell({ primaryId, secondaryId }: { primaryId: number; secondaryId: number }) {
  const primaryName = useCommanderName(primaryId);
  const secondaryName = useCommanderName(secondaryId);

  const primaryFallback =
    typeof primaryId === "number" && Number.isFinite(primaryId) && primaryId > 0
      ? String(primaryId)
      : undefined;
  const secondaryFallback =
    typeof secondaryId === "number" && Number.isFinite(secondaryId) && secondaryId > 0
      ? String(secondaryId)
      : undefined;

  const primaryLabel = primaryName ?? primaryFallback;
  const secondaryLabel = secondaryName ?? secondaryFallback;

  if (!primaryLabel && !secondaryLabel) {
    return <span className="text-zinc-500 dark:text-zinc-400">Unknown pairing</span>;
  }

  return (
    <div className="flex flex-col gap-1">
      {primaryLabel ? <span>{primaryLabel}</span> : null}
      {secondaryLabel ? (
        <span className="text-zinc-600 dark:text-zinc-400">{secondaryLabel}</span>
      ) : null}
    </div>
  );
}

function formatNumber(value: number): string {
  return Number.isFinite(value) ? value.toLocaleString() : "0";
}

function formatPeriod(period: ReturnType<typeof useMyPairings>["period"]) {
  if (!period) {
    return null;
  }

  const start = new Date(period.start);
  const end = new Date(period.end);

  if (Number.isNaN(start.getTime()) || Number.isNaN(end.getTime())) {
    return null;
  }

  const formatter = new Intl.DateTimeFormat("en-US", {
    month: "short",
    day: "numeric",
  });

  return `${formatter.format(start)} – ${formatter.format(end)} UTC`;
}

export default function Page() {
  const { user, loading } = useCurrentUser();
  const governorContext = useContext(GovernorContext);

  if (!governorContext) {
    throw new Error("My Pairings page must be used within a GovernorProvider");
  }

  const { activeGovernor } = governorContext;
  const { data, loading: pairingsLoading, error, period } = useMyPairings();

  const periodLabel = useMemo(() => formatPeriod(period), [period]);

  const content = (() => {
    if (loading) {
      return (
        <p className="mt-8 text-sm text-zinc-500 dark:text-zinc-400">
          Loading your account&hellip;
        </p>
      );
    }

    if (!user) {
      return (
        <p className="mt-8 text-sm text-zinc-500 dark:text-zinc-400">
          You must be logged in to view this page.
        </p>
      );
    }

    if (!activeGovernor) {
      return (
        <p className="mt-8 text-sm text-zinc-500 dark:text-zinc-400">
          You must have a claimed governor to view this page.
        </p>
      );
    }

    return (
      <>
        {periodLabel ? <Text>Period: {periodLabel}</Text> : null}
        <Table dense className="mt-4 [--gutter:--spacing(6)] lg:[--gutter:--spacing(10)]">
          <TableHead>
            <TableRow>
              <TableHeader>Pairing</TableHeader>
              <TableHeader>Fights</TableHeader>
              <TableHeader>Kill Points</TableHeader>
              <TableHeader>Deaths</TableHeader>
              <TableHeader>Severely Wounded</TableHeader>
              <TableHeader>Wounded</TableHeader>
              <TableHeader>Enemy Kill Points</TableHeader>
              <TableHeader>Enemy Deaths</TableHeader>
              <TableHeader>Enemy Severely Wounded</TableHeader>
              <TableHeader>Enemy Wounded</TableHeader>
            </TableRow>
          </TableHead>
          <TableBody>
            {data.map((pairing) => {
              const key = `${pairing.primaryCommanderId}:${pairing.secondaryCommanderId}`;
              return (
                <TableRow key={key}>
                  <TableCell>
                    <CommanderPairCell
                      primaryId={pairing.primaryCommanderId}
                      secondaryId={pairing.secondaryCommanderId}
                    />
                  </TableCell>
                  <TableCell>{formatNumber(pairing.count)}</TableCell>
                  <TableCell>{formatNumber(pairing.totals.killScore)}</TableCell>
                  <TableCell>{formatNumber(pairing.totals.deaths)}</TableCell>
                  <TableCell>{formatNumber(pairing.totals.severelyWounded)}</TableCell>
                  <TableCell>{formatNumber(pairing.totals.wounded)}</TableCell>
                  <TableCell>{formatNumber(pairing.totals.enemyKillScore)}</TableCell>
                  <TableCell>{formatNumber(pairing.totals.enemyDeaths)}</TableCell>
                  <TableCell>{formatNumber(pairing.totals.enemySeverelyWounded)}</TableCell>
                  <TableCell>{formatNumber(pairing.totals.enemyWounded)}</TableCell>
                </TableRow>
              );
            })}
            {pairingsLoading && data.length === 0 ? (
              <TableRow>
                <TableCell colSpan={10}>Loading pairings&hellip;</TableCell>
              </TableRow>
            ) : null}
            {error ? (
              <TableRow>
                <TableCell colSpan={10}>Failed to load pairings: {error}</TableCell>
              </TableRow>
            ) : null}
            {!pairingsLoading && !error && data.length === 0 ? (
              <TableRow>
                <TableCell colSpan={10}>No pairings found for this period.</TableCell>
              </TableRow>
            ) : null}
          </TableBody>
        </Table>
      </>
    );
  })();

  return (
    <>
      <Heading>My Pairings</Heading>
      {content}
    </>
  );
}
