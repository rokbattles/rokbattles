"use client";

import { LoadoutArmamentList } from "@/components/my-pairings/LoadoutArmamentList";
import { LoadoutEquipmentGrid } from "@/components/my-pairings/LoadoutEquipmentGrid";
import { ReportInscriptionBadge } from "@/components/report/ReportInscriptionBadge";
import { Badge } from "@/components/ui/Badge";
import { Subheading } from "@/components/ui/Heading";
import { Text } from "@/components/ui/Text";
import type { LoadoutAggregate } from "@/hooks/usePairings";
import { cn } from "@/lib/cn";

export type LoadoutCard = LoadoutAggregate & {
  label: string;
};

type PairingsLoadoutsProps = {
  pairingsLoading: boolean;
  pairingsError: string | null;
  hasSelectedPairing: boolean;
  loadoutsLoading: boolean;
  loadoutsError: string | null;
  loadoutCards: LoadoutCard[];
  selectedLoadoutKey: string | null;
  onSelectLoadout: (key: string) => void;
};

export function PairingsLoadouts({
  pairingsLoading,
  pairingsError,
  hasSelectedPairing,
  loadoutsLoading,
  loadoutsError,
  loadoutCards,
  selectedLoadoutKey,
  onSelectLoadout,
}: PairingsLoadoutsProps) {
  return (
    <section className="space-y-4">
      <div className="space-y-1">
        <Subheading>Loadouts</Subheading>
        <Text>Scroll sideways to view more loadouts.</Text>
      </div>
      {pairingsLoading ? (
        <Text>Loading pairings...</Text>
      ) : pairingsError ? (
        <Text>{pairingsError}</Text>
      ) : !hasSelectedPairing ? (
        <Text>Select a pairing to get started.</Text>
      ) : loadoutsLoading ? (
        <Text>Loading loadouts...</Text>
      ) : loadoutsError ? (
        <Text>{loadoutsError}</Text>
      ) : (
        <div className="overflow-x-auto pb-4 snap-x snap-mandatory">
          <div className="flex items-stretch gap-4">
            {loadoutCards.map((loadout) => {
              const isSelected = loadout.key === selectedLoadoutKey;

              return (
                <button
                  key={loadout.key}
                  type="button"
                  onClick={() => onSelectLoadout(loadout.key)}
                  aria-pressed={isSelected}
                  className={cn(
                    "snap-start shrink-0 rounded-md border border-zinc-200/70 bg-white/80 p-4 text-left transition hover:border-zinc-300 focus:outline-hidden focus-visible:ring-2 focus-visible:ring-blue-500/60 dark:border-white/10 dark:bg-white/5 dark:hover:border-white/20",
                    "flex flex-col items-stretch justify-start self-stretch",
                    "w-full sm:w-[calc((100%-1rem)/2)] lg:w-[calc((100%-2rem)/3)]"
                  )}
                >
                  <div className="flex items-start justify-between gap-2">
                    <span className="text-sm font-semibold text-zinc-950 dark:text-white">
                      {loadout.label}
                    </span>
                    <Badge
                      color="blue"
                      className={cn(!isSelected && "invisible")}
                      aria-hidden={!isSelected}
                    >
                      Selected
                    </Badge>
                  </div>

                  <div className="mt-4 space-y-4">
                    <LoadoutEquipmentGrid tokens={loadout.loadout.equipment} />

                    <div className="space-y-2">
                      <p className="text-xs font-semibold uppercase tracking-wide text-zinc-500">
                        Inscriptions
                      </p>
                      <div className="flex min-h-5 flex-wrap gap-1.5">
                        {loadout.loadout.inscriptions.map((id) => (
                          <ReportInscriptionBadge key={id} id={id} />
                        ))}
                      </div>
                    </div>

                    <div className="space-y-2">
                      <p className="text-xs font-semibold uppercase tracking-wide text-zinc-500">
                        Armament buffs
                      </p>
                      <LoadoutArmamentList armaments={loadout.loadout.armaments} />
                    </div>
                  </div>
                </button>
              );
            })}
          </div>
        </div>
      )}
    </section>
  );
}
