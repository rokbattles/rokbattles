"use client";

import { useSearchParams } from "next/navigation";
import { type ReactNode, use } from "react";
import { ResourcesFiltersClient } from "@/components/account-resources/resources-filters-client";
import { CommanderIcon } from "@/components/commander-icon";
import { PairingsFilters } from "@/components/my-pairings/pairings-filters";
import { Button } from "@/components/ui/button";
import { Subheading } from "@/components/ui/heading";
import { Text } from "@/components/ui/text";
import { buildPairingsRangeParams } from "@/lib/pairings";
import { commanderName } from "@/lib/pairings/analytics";
import { ACTIVITY_LABELS, BATTLE_LABELS, readExclusions } from "@/lib/pairings/filters";
import { resolveResourceRange } from "@/lib/resources/date-range";
import { GovernorContext } from "@/providers/governor-context";

export function usePairingsContext() {
  const context = use(GovernorContext);
  if (!context) throw new Error("Pairings require a GovernorProvider");

  const params = useSearchParams();
  const today = new Date().toISOString().slice(0, 10);
  const range = resolveResourceRange(params.get("start"), params.get("end"), today);
  const query = buildPairingsRangeParams({
    startDate: range.start,
    endDate: range.end,
    excludeActivities: readExclusions(params.get("excludeActivities"), ACTIVITY_LABELS),
    excludeBattles: readExclusions(params.get("excludeBattles"), BATTLE_LABELS),
  });

  return { governorId: context.activeGovernor?.governorId, range, today, query, params };
}

export function PairingsToolbar({
  range,
  today,
  compact = false,
}: {
  compact?: boolean;
  range: { start: string; end: string };
  today: string;
}) {
  return (
    <div className={compact ? "flex flex-wrap items-center gap-3" : "space-y-4"}>
      <ResourcesFiltersClient
        startDate={range.start}
        endDate={range.end}
        maxDate={today}
        rangeLabel="Pairing"
        compact={compact}
      />
      <PairingsFilters compact={compact} />
    </div>
  );
}

export function PairingIcons({
  primary,
  secondary,
  large = false,
  chart = false,
  compact = false,
}: {
  primary: number;
  secondary: number;
  large?: boolean;
  chart?: boolean;
  compact?: boolean;
}) {
  const variant = large ? "large" : chart ? "chart" : "default";
  const styles = {
    large: { icon: "size-12", spacing: "-space-x-3", sizes: "48px" },
    chart: compact
      ? { icon: "size-7", spacing: "-space-x-2", sizes: "40px" }
      : { icon: "size-10", spacing: "-space-x-3", sizes: "40px" },
    default: { icon: "size-8", spacing: "-space-x-2", sizes: "40px" },
  }[variant];

  return (
    <span className={`flex shrink-0 ${styles.spacing}`}>
      <CommanderIcon
        id={primary}
        alt={commanderName(primary)}
        className={styles.icon}
        sizes={styles.sizes}
        loading={chart ? "eager" : "lazy"}
      />
      {secondary > 0 ? (
        <CommanderIcon
          id={secondary}
          alt={commanderName(secondary)}
          className={styles.icon}
          sizes={styles.sizes}
          loading={chart ? "eager" : "lazy"}
        />
      ) : null}
    </span>
  );
}

export function PairingIdentity({
  primary,
  secondary,
  large = false,
}: {
  primary: number;
  secondary: number;
  large?: boolean;
}) {
  return (
    <span className="flex min-w-0 items-center gap-3">
      <PairingIcons primary={primary} secondary={secondary} large={large} />
      <span className="min-w-0">
        <span className="block font-medium text-white">{commanderName(primary)}</span>
        <span className="block text-xs text-zinc-400">
          {secondary > 0 ? commanderName(secondary) : "Solo commander"}
        </span>
      </span>
    </span>
  );
}

export function PairingsState({
  loading,
  error,
  retry,
  children,
}: {
  loading?: boolean;
  error?: string | null;
  retry?: () => void;
  children?: ReactNode;
}) {
  if (error)
    return (
      <div className="space-y-3" role="alert">
        <Text>{error}</Text>
        {retry ? (
          <Button outline onClick={retry}>
            Try again
          </Button>
        ) : null}
      </div>
    );
  if (loading)
    return (
      <div className="rounded-md border border-zinc-800 px-6 py-16 text-center" role="status">
        <Text>Loading pairings…</Text>
      </div>
    );
  return (
    <div className="rounded-md border border-dashed border-zinc-700 px-6 py-10 text-center">
      <Subheading>{children ?? "No pairings in this date range."}</Subheading>
    </div>
  );
}
