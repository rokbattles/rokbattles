import { Forward, Funnel, Star, Swords } from "lucide-react";

function EquipmentCell({ id }: { id?: number }) {
  return (
    <div className="flex items-center justify-center">
      <div className="h-12 w-12 rounded-md border border-white/10 bg-zinc-900/60 text-[10px] font-semibold text-zinc-200 flex items-center justify-center">
        {id ?? "—"}
      </div>
    </div>
  );
}

function EquipmentGridFixed({ slots }: { slots: Record<number, number> }) {
  return (
    <div className="inline-grid grid-cols-[auto_auto_auto] gap-2">
      <div />
      <EquipmentCell id={slots[2]} />
      <div />
      <EquipmentCell id={slots[1]} />
      <EquipmentCell id={slots[3]} />
      <EquipmentCell id={slots[4]} />
      <EquipmentCell id={slots[7]} />
      <EquipmentCell id={slots[5]} />
      <EquipmentCell id={slots[8]} />
      <div />
      <EquipmentCell id={slots[6]} />
      <div />
    </div>
  );
}

type Rarity = "common" | "rare" | "special";

function InscriptionBadge({ label, rarity }: { label: string; rarity: Rarity }) {
  const base =
    "inline-flex items-center rounded-md px-2.5 py-1 text-[10px] font-semibold ring-1 ring-inset";
  if (rarity === "common") {
    return (
      <span className={`${base} bg-white text-zinc-800 ring-zinc-200`} title={label}>
        {label}
      </span>
    );
  }
  if (rarity === "rare") {
    return (
      <span
        className={`${base} text-white ring-blue-400/40 bg-[radial-gradient(ellipse_at_center,theme(colors.blue.400),theme(colors.blue.600))]`}
        title={label}
      >
        {label}
      </span>
    );
  }
  // special
  return (
    <span
      className={`${base} text-white ring-amber-400/40 bg-[radial-gradient(ellipse_at_center,theme(colors.amber.400),theme(colors.amber.500))]`}
      title={label}
    >
      {label}
    </span>
  );
}

function BuffRow({
  label,
  value,
  align = "left",
}: {
  label: string;
  value: string;
  align?: "left" | "right";
}) {
  return (
    <div
      className={`grid grid-cols-[1fr_auto] items-baseline ${align === "right" ? "text-right" : ""}`}
    >
      <div className="text-sm text-zinc-300">{label}</div>
      <div className="ml-4 text-sm tabular-nums">{value}</div>
    </div>
  );
}

export default function LivePage() {
  const defenderEquip = {
    1: 20013,
    2: 20014,
    3: 20026,
    4: 20029,
    5: 20032,
    6: 20034,
    7: 20037,
    8: 20038,
  } as const;

  const attackerEquip = {
    1: 20019,
    2: 20020,
    3: 20021,
    4: 20022,
    5: 20023,
    6: 20024,
    7: 20037,
    8: 20041,
  } as const;

  const defenderInscriptions: Array<{ label: string; rarity: Rarity }> = [
    { label: "Deflector", rarity: "common" },
    { label: "Toppler", rarity: "special" },
    { label: "Raider", rarity: "rare" },
    { label: "Hardheaded", rarity: "rare" },
  ];

  const attackerInscriptions: Array<{ label: string; rarity: Rarity }> = [
    { label: "Counterer", rarity: "common" },
    { label: "Hunter", rarity: "special" },
    { label: "Unstoppable", rarity: "special" },
    { label: "Balanced", rarity: "special" },
    { label: "Intrepid", rarity: "special" },
  ];

  const defenderBuffs = [
    { label: "Infantry Attack", value: "6.9%" },
    { label: "Infantry Defense", value: "9.6%" },
    { label: "Infantry Health", value: "10.5%" },
    { label: "March Speed (Infantry)", value: "1.9%" },
  ] as const;

  const attackerBuffs = [
    { label: "Archer Attack", value: "10.3%" },
    { label: "Archer Defense", value: "12.2%" },
    { label: "Archer Health", value: "8.6%" },
    { label: "All Damage", value: "2%" },
  ] as const;

  return (
    <div className="relative isolate flex min-h-svh w-full max-lg:flex-col bg-zinc-900 lg:bg-zinc-950">
      <aside className="fixed inset-y-0 left-0 w-96 max-lg:hidden">
        <div className="flex h-full flex-col">
          <div className="flex items-center justify-between p-5">
            <div className="flex items-center gap-2">
              <span className="relative flex size-2.5">
                <span className="absolute inline-flex h-full w-full rounded-full bg-red-400 opacity-75 animate-ping"></span>
                <span className="relative inline-flex rounded-full size-2.5 bg-red-500"></span>
              </span>
              <span className="text-zinc-300 text-sm">Live mode</span>
            </div>
            <div className="flex items-center gap-1.5 text-sm text-zinc-300">
              <Funnel className="size-3.5 text-zinc-400" aria-hidden="true" />
              <span className="select-none">Filter</span>
            </div>
          </div>
          <div className="border-t border-white/10" />
          <div className="min-h-0 flex-1 overflow-y-auto">
            <div className="flex items-center justify-between py-3 px-5 hover:bg-zinc-800 cursor-pointer transition">
              <div className="w-36">
                <div className="truncate text-sm font-medium text-zinc-100">Gorgo</div>
                <div className="truncate text-xs text-zinc-400">Heraclius</div>
              </div>
              <div className="mx-2 flex shrink-0 items-center justify-center">
                <Swords className="size-5 text-zinc-300" aria-hidden="true" />
              </div>
              <div className="w-36 text-right">
                <div className="truncate text-sm font-medium text-zinc-100">Ashurbanipal</div>
                <div className="truncate text-xs text-zinc-400">Zhuge Liang</div>
              </div>
            </div>
            {Array.from({ length: 20 }).map((_, i) => (
              <div key={i} className="flex items-center justify-between py-3 px-5">
                <div className="w-36">
                  <div className="h-3.5 w-24 animate-pulse rounded bg-zinc-800/80" />
                  <div className="mt-1 h-2.5 w-16 animate-pulse rounded bg-zinc-800/70" />
                </div>
                <div className="mx-2 size-5 rounded bg-zinc-800/80" />
                <div className="w-36 text-right">
                  <div className="ml-auto h-3.5 w-28 animate-pulse rounded bg-zinc-800/80" />
                  <div className="ml-auto mt-1 h-2.5 w-20 animate-pulse rounded bg-zinc-800/70" />
                </div>
              </div>
            ))}
          </div>
        </div>
      </aside>
      <main className="flex flex-1 flex-col lg:min-w-0 lg:pl-96">
        <div className="grow p-6 lg:bg-zinc-900 lg:shadow-xs lg:ring-1 lg:ring-white/10">
          <div className="max-w-6xl mx-auto">
            <div className="flex items-center justify-between gap-3">
              <h1 className="text-xl font-semibold text-zinc-100">Battle Report</h1>
              <div className="flex items-center gap-2">
                <button
                  type="button"
                  className="group inline-flex items-center gap-1.5 rounded-md border border-white/10 bg-zinc-900/60 px-2.5 py-1.5 text-sm text-zinc-200 hover:bg-zinc-800/60 focus:outline-none focus:ring-2 focus:ring-white/20"
                >
                  <Star
                    className="size-4 text-zinc-300 group-hover:text-amber-400"
                    aria-hidden="true"
                  />
                  <span className="tabular-nums text-zinc-300">128</span>
                </button>
                <button
                  type="button"
                  className="inline-flex items-center gap-1.5 rounded-md border border-white/10 bg-zinc-900/60 px-2.5 py-1.5 text-sm text-zinc-200 hover:bg-zinc-800/60 focus:outline-none focus:ring-2 focus:ring-white/20"
                >
                  <Forward className="size-4 text-zinc-300" aria-hidden="true" />
                  <span className="hidden sm:inline">Share</span>
                </button>
              </div>
            </div>
            <div className="mt-4 mb-8 border-t border-white/10" />
            <div className="mb-4 flex justify-center items-center gap-4 text-xs text-zinc-500">
              <span>UTC 07/12 18:29</span>
              <span>X: 584 Y: 660</span>
            </div>
            <div className="grid gap-4 lg:grid-cols-4">
              <div className="relative pr-4">
                <div className="truncate font-semibold text-zinc-100 mb-1">[SO4L] agent G</div>
                <div className="text-xs text-zinc-500">X: 584 Y: 660</div>
                <div className="mt-4">
                  <EquipmentGridFixed slots={defenderEquip} />
                </div>
                <div className="mt-4">
                  <div className="flex flex-wrap gap-1.5">
                    {defenderInscriptions.map((it) => (
                      <InscriptionBadge key={it.label} label={it.label} rarity={it.rarity} />
                    ))}
                  </div>
                </div>
                <div className="mt-4">
                  <div className="space-y-1">
                    {defenderBuffs.map((b) => (
                      <BuffRow key={b.label} label={b.label} value={b.value} />
                    ))}
                  </div>
                </div>
              </div>
              <div className="relative lg:col-span-2 px-4">
                <div className="text-center text-zinc-100 font-semibold mb-4">Battle Results</div>
                <div className="mb-4 grid grid-cols-2 gap-4">
                  <div>
                    <ul className="space-y-1.5">
                      <li className="flex items-center gap-2">
                        <span className="text-sm text-zinc-200">Gorgo</span>
                        <span className="rounded-sm bg-zinc-800/70 px-1.5 py-0.5 text-[10px] text-zinc-300">
                          Lv60
                        </span>
                      </li>
                      <li className="flex items-center gap-2">
                        <span className="text-sm text-zinc-200">Heraclius</span>
                        <span className="rounded-sm bg-zinc-800/70 px-1.5 py-0.5 text-[10px] text-zinc-300">
                          Lv60
                        </span>
                      </li>
                    </ul>
                  </div>
                  <div className="lg:text-right">
                    <ul className="space-y-1.5">
                      <li className="flex items-center gap-2 lg:justify-end">
                        <span className="rounded-sm bg-zinc-800/70 px-1.5 py-0.5 text-[10px] text-zinc-300">
                          Lv60
                        </span>
                        <span className="text-sm text-zinc-200">Ashurbanipal</span>
                      </li>
                      <li className="flex items-center gap-2 lg:justify-end">
                        <span className="rounded-sm bg-zinc-800/70 px-1.5 py-0.5 text-[10px] text-zinc-300">
                          Lv60
                        </span>
                        <span className="text-sm text-zinc-200">Zhuge Liang</span>
                      </li>
                    </ul>
                  </div>
                </div>
                <div className="space-y-2">
                  <div className="flex items-center justify-between">
                    <div className="w-32">
                      <span className="text-sm tabular-nums">6,316,555</span>
                    </div>
                    <div className="mx-2 flex shrink-0 items-center justify-center">
                      <span className="text-sm text-zinc-300">Units</span>
                    </div>
                    <div className="w-32 text-right">
                      <span className="text-sm tabular-nums">3,599,999</span>
                    </div>
                  </div>
                  <div className="flex items-center justify-between">
                    <div className="w-32">
                      <span className="text-sm tabular-nums">0</span>
                    </div>
                    <div className="mx-2 flex shrink-0 items-center justify-center">
                      <span className="text-sm text-zinc-300">Heal</span>
                    </div>
                    <div className="w-32 text-right">
                      <span className="text-sm tabular-nums">0</span>
                    </div>
                  </div>
                  <div className="flex items-center justify-between">
                    <div className="w-32">
                      <span className="text-sm tabular-nums">3,439</span>
                    </div>
                    <div className="mx-2 flex shrink-0 items-center justify-center">
                      <span className="text-sm text-zinc-300">Dead</span>
                    </div>
                    <div className="w-32 text-right">
                      <span className="text-sm tabular-nums">139,437</span>
                    </div>
                  </div>
                  <div className="flex items-center justify-between">
                    <div className="w-32">
                      <span className="text-sm tabular-nums">121,447</span>
                    </div>
                    <div className="mx-2 flex shrink-0 items-center justify-center">
                      <span className="text-sm text-zinc-300">Severely Wounded</span>
                    </div>
                    <div className="w-32 text-right">
                      <span className="text-sm tabular-nums">0</span>
                    </div>
                  </div>
                  <div className="flex items-center justify-between">
                    <div className="w-32">
                      <span className="text-sm tabular-nums">826,540</span>
                    </div>
                    <div className="mx-2 flex shrink-0 items-center justify-center">
                      <span className="text-sm text-zinc-300">Slightly Wounded</span>
                    </div>
                    <div className="w-32 text-right">
                      <span className="text-sm tabular-nums">779,112</span>
                    </div>
                  </div>
                  <div className="flex items-center justify-between">
                    <div className="w-32">
                      <span className="text-sm tabular-nums">5,315,229</span>
                    </div>
                    <div className="mx-2 flex shrink-0 items-center justify-center">
                      <span className="text-sm text-zinc-300">Remaining</span>
                    </div>
                    <div className="w-32 text-right">
                      <span className="text-sm tabular-nums">2,017,456</span>
                    </div>
                  </div>
                  <div className="flex items-center justify-between">
                    <div className="w-32">
                      <span className="text-sm tabular-nums">50,000</span>
                    </div>
                    <div className="mx-2 flex shrink-0 items-center justify-center">
                      <span className="text-sm text-zinc-300">Watchtower Damage</span>
                    </div>
                    <div className="w-32 text-right">
                      <span className="text-sm tabular-nums"></span>
                    </div>
                  </div>
                  <div className="flex items-center justify-between">
                    <div className="w-32">
                      <span className="text-sm tabular-nums">2,024,818</span>
                    </div>
                    <div className="mx-2 flex shrink-0 items-center justify-center">
                      <span className="text-sm text-zinc-300">Kill Points</span>
                    </div>
                    <div className="w-32 text-right">
                      <span className="text-sm tabular-nums">2,302,550</span>
                    </div>
                  </div>
                </div>
              </div>
              <div className="relative pl-4 lg:text-right">
                <div className="truncate font-semibold text-zinc-100 lg:text-right mb-1">
                  [NW25] 西伯利亞狼
                </div>
                <div className="text-xs text-zinc-500 lg:text-right">X: 573 Y: 660</div>
                <div className="mt-4">
                  <EquipmentGridFixed slots={attackerEquip} />
                </div>
                <div className="mt-4">
                  <div className="inline-flex flex-wrap gap-1.5 flex-row-reverse">
                    {attackerInscriptions.map((it) => (
                      <InscriptionBadge key={it.label} label={it.label} rarity={it.rarity} />
                    ))}
                  </div>
                </div>
                <div className="mt-4 lg:text-right">
                  <div className="space-y-1">
                    {attackerBuffs.map((b) => (
                      <BuffRow key={b.label} label={b.label} value={b.value} align="right" />
                    ))}
                  </div>
                </div>
              </div>
            </div>
          </div>
        </div>
      </main>
    </div>
  );
}
