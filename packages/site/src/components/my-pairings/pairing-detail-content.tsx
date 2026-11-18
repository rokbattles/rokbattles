"use client";

import { ArrowLeft } from "lucide-react";
import { usePathname, useRouter } from "next/navigation";
import type { ReactNode } from "react";
import { PairingLoadoutComparison } from "@/components/my-pairings/pairing-loadout-comparison";
import { PairingMatchups } from "@/components/my-pairings/pairing-matchups";
import { PairingPerformance } from "@/components/my-pairings/pairing-performance";
import {
  PairingIdentity,
  PairingsState,
  PairingsToolbar,
  usePairingsContext,
} from "@/components/my-pairings/pairings-shared";
import { Link } from "@/components/ui/link";
import { usePairingsQuery } from "@/hooks/use-pairings";
import type { PairingDetailResponse, PairingMatchupsResponse } from "@/lib/pairings";
import { formatExact, MIN_PAIRING_BATTLES } from "@/lib/pairings/analytics";

const TABS = [
  ["performance", "Performance"],
  ["matchups", "Matchups"],
  ["loadouts", "Loadout"],
] as const;

export function PairingDetailContent({
  primary,
  secondary,
}: {
  primary: number;
  secondary: number;
}) {
  const { governorId, range, today, query, params } = usePairingsContext();
  const router = useRouter();
  const pathname = usePathname();

  const tab = TABS.find(([value]) => value === params.get("tab"))?.[0] ?? "performance";
  const detail = usePairingDetail(
    governorId,
    query.toString(),
    primary,
    secondary,
    params.get("loadout")
  );

  function changeLoadout(key: string | null) {
    const next = new URLSearchParams(params.toString());
    if (key) next.set("loadout", key);
    else next.delete("loadout");
    router.replace(`${pathname}?${next}`, { scroll: false });
  }

  return (
    <div className="relative min-w-0 max-w-full space-y-6 overflow-x-clip">
      <Link
        href={`/account/pairings?${query}`}
        prefetch={false}
        className="inline-flex items-center gap-2 text-sm text-zinc-400 hover:text-white"
      >
        <ArrowLeft className="size-3.5" aria-hidden="true" />
        My Pairings
      </Link>
      <h1>
        <PairingIdentity primary={primary} secondary={secondary} large />
      </h1>
      <PairingsToolbar range={range} today={today} compact />
      <PairingDetailBody
        {...detail}
        tab={tab}
        params={params.toString()}
        onSelect={changeLoadout}
      />
    </div>
  );
}

function usePairingDetail(
  governorId: number | undefined,
  query: string,
  primary: number,
  secondary: number,
  loadoutKey: string | null
) {
  const request = new URLSearchParams(query);
  request.set("primary", String(primary));
  request.set("secondary", String(secondary));

  const result = usePairingsQuery<PairingDetailResponse>(
    governorId ? `/proxy/v1/governor/${governorId}/pairings/detail?${request}` : null
  );

  const data = result.data;
  const selected = data?.loadouts.find((item) => item.key === loadoutKey)?.key ?? null;
  if (selected) request.set("loadoutKey", selected);

  const matchups = usePairingsQuery<PairingMatchupsResponse>(
    governorId && data?.pairing && data.pairing.battles >= MIN_PAIRING_BATTLES
      ? `/proxy/v1/governor/${governorId}/pairings/matchups?${request}`
      : null
  );

  return {
    result,
    selected,
    matchups,
    performanceUrl: governorId
      ? `/proxy/v1/governor/${governorId}/pairings/performance?${request}`
      : null,
    selectionKey: `${governorId}:${selected}:${query}`,
  };
}

function PairingDetailBody({
  result,
  selected,
  matchups,
  performanceUrl,
  selectionKey,
  tab,
  params,
  onSelect,
}: ReturnType<typeof usePairingDetail> & {
  tab: string;
  params: string;
  onSelect: (key: string | null) => void;
}) {
  if (result.loading) return <PairingsState loading />;
  if (result.error) return <PairingsState error={result.error} retry={result.retry} />;

  const data = result.data;
  if (!data?.pairing || data.pairing.battles < MIN_PAIRING_BATTLES) {
    return <PairingsState>No pairing with 100+ battles in this range.</PairingsState>;
  }

  let panel: ReactNode;
  if (tab === "matchups") {
    panel = (
      <PairingMatchups
        key={selectionKey}
        items={matchups.data?.items ?? []}
        loading={matchups.loading}
        error={matchups.error}
        retry={matchups.retry}
      />
    );
  } else if (tab === "loadouts") {
    panel = (
      <PairingLoadoutComparison
        loadouts={data.loadouts}
        selected={selected}
        allLoadouts={data.pairing}
        onSelect={onSelect}
      />
    );
  } else {
    panel = <PairingPerformance key={selectionKey} url={performanceUrl} />;
  }

  return (
    <>
      <PairingTabs
        tab={tab}
        params={params}
        loadoutCount={data.loadouts.length}
        matchupCount={matchups.data?.items.length}
      />
      {panel}
    </>
  );
}

function PairingTabs({
  tab,
  params,
  loadoutCount,
  matchupCount,
}: {
  tab: string;
  params: string;
  loadoutCount: number;
  matchupCount: number | undefined;
}) {
  return (
    <nav className="flex gap-6 border-b border-zinc-800" aria-label="Pairing details">
      {TABS.map(([value, label]) => {
        const next = new URLSearchParams(params);
        next.set("tab", value);
        const count = value === "loadouts" ? loadoutCount : matchupCount;
        return (
          <Link
            href={`?${next}`}
            key={value}
            prefetch={false}
            replace
            scroll={false}
            aria-current={tab === value ? "page" : undefined}
            className={`-mb-px border-b-2 py-3 text-sm font-medium ${tab === value ? "border-white text-white" : "border-transparent text-zinc-400 hover:text-white"}`}
          >
            {label}
            {value !== "performance" ? (
              <>
                {" "}
                <span className="ml-1 rounded bg-white/5 px-1.5 py-0.5 text-xs font-normal text-zinc-400 tabular-nums">
                  {count === undefined ? "…" : formatExact(count)}
                </span>
              </>
            ) : null}
          </Link>
        );
      })}
    </nav>
  );
}
