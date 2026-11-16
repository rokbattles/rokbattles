import type { JSX } from "react";
import { BattleCard } from "@/components/(app)/overlay/battle-card";
import type { BattleCardData, OverlayBranding, OverlayCommander } from "@/lib/(app)/types/overlay";

type BattleCardsProps = {
  battles: BattleCardData[];
  commanders: Record<string, OverlayCommander>;
  branding: OverlayBranding;
  status: string;
};

export function BattleCards({
  battles,
  commanders,
  branding = "text_watermark",
  status,
}: BattleCardsProps): JSX.Element {
  return (
    <main
      aria-label="Recent battles"
      data-status={status}
      className="flex w-[390px] max-w-screen flex-col gap-1 p-1 text-[#fff9e9] tabular-nums [font-family:Segoe_UI,Arial,sans-serif]"
    >
      {battles.map((battle) => (
        <BattleCard key={battle.id} battle={battle} commanders={commanders} branding={branding} />
      ))}
    </main>
  );
}
