"use client";

import { Circle, CircleCheck, Layers } from "lucide-react";
import { LoadoutArmamentList } from "@/components/my-pairings/loadout-armament-list";
import { LoadoutEquipmentGrid } from "@/components/my-pairings/loadout-equipment-grid";
import { ReportInscriptionBadge } from "@/components/report/report-inscription-badge";
import type { PairingDetailLoadout, PairingStats } from "@/lib/pairings";
import { formatCompact, formatExact } from "@/lib/pairings/analytics";
import { formatDetailTrade } from "@/lib/pairings/detail";

export function PairingLoadoutComparison({
  loadouts,
  selected,
  allLoadouts,
  onSelect,
}: {
  loadouts: PairingDetailLoadout[];
  selected: string | null;
  allLoadouts: PairingStats;
  onSelect: (key: string | null) => void;
}) {
  const cards = [
    { key: null, label: "All Loadouts", stats: allLoadouts, loadout: null },
    ...loadouts.map((item, index) => ({ ...item, label: `Loadout ${index + 1}` })),
  ];

  return (
    <section className="grid gap-4 md:grid-cols-2 xl:grid-cols-3" aria-label="Loadout selection">
      {cards.map((item) => {
        const isSelected = selected === item.key;
        return (
          <button
            key={item.key ?? "all"}
            type="button"
            aria-label={`Select ${item.label}`}
            aria-pressed={isSelected}
            onClick={() => onSelect(item.key)}
            className={`flex min-w-0 flex-col rounded-md border p-4 text-left transition focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-blue-500 sm:p-5 ${isSelected ? "border-zinc-500 bg-white/5" : "border-white/10 bg-white/5 hover:border-white/30"}`}
          >
            <div className="flex items-center justify-between gap-3">
              <span className="text-sm font-semibold text-white">{item.label}</span>
              {isSelected ? (
                <CircleCheck className="size-5 shrink-0 text-white" aria-hidden="true" />
              ) : (
                <Circle className="size-5 shrink-0 text-zinc-500" aria-hidden="true" />
              )}
            </div>
            <dl className="mt-4 grid grid-cols-2 gap-4 border-b border-white/10 pb-4 2xl:grid-cols-4">
              {[
                { label: "Battles", value: formatExact(item.stats.battles) },
                {
                  label: "KP Gain",
                  value: formatCompact(item.stats.kpGain),
                  exact: item.stats.kpGain,
                },
                {
                  label: "KP Loss",
                  value: formatCompact(item.stats.kpLoss),
                  exact: item.stats.kpLoss,
                },
                { label: "Trade %", value: formatDetailTrade(item.stats) },
              ].map((metric) => (
                <div
                  key={metric.label}
                  title={metric.exact == null ? undefined : formatExact(metric.exact)}
                >
                  <dt className="text-xs text-zinc-400">{metric.label}</dt>
                  <dd className="mt-1 text-base font-medium text-white tabular-nums">
                    {metric.value}
                  </dd>
                </div>
              ))}
            </dl>
            {item.loadout ? (
              <div className="mt-5 w-full space-y-5">
                <LoadoutEquipmentGrid tokens={item.loadout.equipment} />
                <div className="space-y-2">
                  <p className="text-xs font-semibold uppercase tracking-wide text-zinc-500">
                    Inscriptions
                  </p>
                  <div className="flex min-h-5 flex-wrap gap-1.5">
                    {item.loadout.inscriptions.length ? (
                      item.loadout.inscriptions.map((id) => (
                        <ReportInscriptionBadge key={id} id={id} />
                      ))
                    ) : (
                      <span className="text-xs text-zinc-400">No inscriptions recorded.</span>
                    )}
                  </div>
                </div>
                <div className="space-y-2">
                  <p className="text-xs font-semibold uppercase tracking-wide text-zinc-500">
                    Armament buffs
                  </p>
                  {item.loadout.armaments.length ? (
                    <LoadoutArmamentList armaments={item.loadout.armaments} />
                  ) : (
                    <p className="text-xs text-zinc-400">No armament buffs recorded.</p>
                  )}
                </div>
              </div>
            ) : (
              <div className="flex w-full flex-1 flex-col items-center justify-center gap-3 py-12 text-center text-zinc-400">
                <Layers className="size-8 text-zinc-500" aria-hidden="true" />
                <span className="max-w-56 text-sm">
                  Combined results across every loadout for this pairing.
                </span>
              </div>
            )}
          </button>
        );
      })}
    </section>
  );
}
