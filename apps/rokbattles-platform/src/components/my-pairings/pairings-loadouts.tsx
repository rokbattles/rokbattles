"use client";

import { useTranslations } from "next-intl";
import { LoadoutArmamentList } from "@/components/my-pairings/loadout-armament-list";
import { LoadoutEquipmentGrid } from "@/components/my-pairings/loadout-equipment-grid";
import { ReportInscriptionBadge } from "@/components/report/report-inscription-badge";
import { Badge } from "@/components/ui/badge";
import { Subheading } from "@/components/ui/heading";
import { Text } from "@/components/ui/text";
import type { LoadoutAggregate } from "@/hooks/use-pairings";
import { cn } from "@/lib/cn";

export type LoadoutCard = LoadoutAggregate & {
  label: string;
};

interface PairingsLoadoutsProps {
  pairingsLoading: boolean;
  pairingsError: string | null;
  hasSelectedPairing: boolean;
  loadoutsLoading: boolean;
  loadoutsError: string | null;
  loadoutCards: LoadoutCard[];
  selectedLoadoutKey: string | null;
  onSelectLoadout: (key: string) => void;
}

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
  const t = useTranslations("pairings");

  return (
    <section className="space-y-4">
      <div className="space-y-1">
        <Subheading>{t("loadouts.title")}</Subheading>
        <Text>{t("loadouts.hint")}</Text>
      </div>
      {pairingsLoading ? (
        <Text>{t("states.loadingPairings")}</Text>
      ) : pairingsError ? (
        <Text>{pairingsError}</Text>
      ) : hasSelectedPairing ? (
        loadoutsLoading ? (
          <Text>{t("loadouts.states.loadingLoadouts")}</Text>
        ) : loadoutsError ? (
          <Text>{loadoutsError}</Text>
        ) : (
          <div className="snap-x snap-mandatory overflow-x-auto pb-4">
            <div className="flex items-stretch gap-4">
              {loadoutCards.map((loadout) => {
                const isSelected = loadout.key === selectedLoadoutKey;

                return (
                  <button
                    aria-pressed={isSelected}
                    className={cn(
                      "shrink-0 snap-start rounded-md border border-zinc-200/70 bg-white/80 p-4 text-left transition hover:border-zinc-300 focus:outline-hidden focus-visible:ring-2 focus-visible:ring-blue-500/60 dark:border-white/10 dark:bg-white/5 dark:hover:border-white/20",
                      "flex flex-col items-stretch justify-start self-stretch",
                      "w-full sm:w-[calc((100%-1rem)/2)] lg:w-[calc((100%-2rem)/3)]"
                    )}
                    key={loadout.key}
                    onClick={() => onSelectLoadout(loadout.key)}
                    type="button"
                  >
                    <div className="flex items-start justify-between gap-2">
                      <span className="font-semibold text-sm text-zinc-950 dark:text-white">
                        {loadout.label}
                      </span>
                      <Badge
                        aria-hidden={!isSelected}
                        className={cn(!isSelected && "invisible")}
                        color="blue"
                      >
                        {t("labels.selected")}
                      </Badge>
                    </div>

                    <div className="mt-4 space-y-4">
                      <LoadoutEquipmentGrid
                        tokens={loadout.loadout.equipment}
                      />

                      <div className="space-y-2">
                        <p className="font-semibold text-xs text-zinc-500 uppercase tracking-wide">
                          {t("labels.inscriptions")}
                        </p>
                        <div className="flex min-h-5 flex-wrap gap-1.5">
                          {loadout.loadout.inscriptions.map((id) => (
                            <ReportInscriptionBadge id={id} key={id} />
                          ))}
                        </div>
                      </div>

                      <div className="space-y-2">
                        <p className="font-semibold text-xs text-zinc-500 uppercase tracking-wide">
                          {t("labels.armamentBuffs")}
                        </p>
                        <LoadoutArmamentList
                          armaments={loadout.loadout.armaments}
                        />
                      </div>
                    </div>
                  </button>
                );
              })}
            </div>
          </div>
        )
      ) : (
        <Text>{t("states.selectPairing")}</Text>
      )}
    </section>
  );
}
