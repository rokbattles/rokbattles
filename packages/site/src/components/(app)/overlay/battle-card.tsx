import { cn } from "cn";
import Image from "next/image";
import type { JSX } from "react";
import { CommanderPortrait } from "@/components/(app)/overlay/commander-portrait";
import type { BattleCardData, OverlayBranding, OverlayCommander } from "@/lib/(app)/types/overlay";

type BattleCardProps = {
  battle: BattleCardData;
  commanders: Record<string, OverlayCommander>;
  branding: OverlayBranding;
};

const numbers = new Intl.NumberFormat("en-US");
const labels = { victory: "Victory", defeat: "Defeat", battle: "Battle", unknown: "Battle" };
const watermarks = {
  icon_watermark: {
    src: "https://cdn.rokbattles.com/assets/rokbattles-icon-white.svg",
    width: 32,
    height: 32,
  },
  text_watermark: {
    src: "https://cdn.rokbattles.com/assets/rokbattles-wordmark-white.svg",
    width: 112,
    height: 13,
  },
};
export function BattleCard({ battle, commanders, branding }: BattleCardProps): JSX.Element {
  const trade = battle.tradePercent === null ? "∞" : numbers.format(battle.tradePercent);
  const watermark = watermarks[branding] ?? watermarks.text_watermark;

  return (
    <article
      data-outcome={battle.outcome}
      className={cn(
        "relative flex min-h-[94px] animate-battle-arrive items-center rounded-[9px] border-8 border-transparent px-2.5 py-1.5",
        "bg-[rgb(73_52_21/75%)] bg-clip-padding drop-shadow-[0_2px_2px_#0006] motion-reduce:animate-none",
        "[border-image:url('https://cdn.rokbattles.com/game/sprites/img_mailbg4.png')_14_fill_/_14px_/_0_stretch]"
      )}
    >
      <Image
        src={watermark.src}
        alt=""
        width={watermark.width}
        height={watermark.height}
        unoptimized
        className="pointer-events-none absolute top-2 right-2 h-auto opacity-15"
      />
      <div className="relative h-[62px] w-[94px] shrink-0">
        <CommanderPortrait
          commander={commanders[battle.commanders.secondaryCommanderId]}
          awakened={battle.commanders.secondaryCommanderAwakened}
          secondary
        />
        <CommanderPortrait
          commander={commanders[battle.commanders.primaryCommanderId]}
          awakened={battle.commanders.primaryCommanderAwakened}
        />
      </div>
      <div className="relative min-w-0">
        <h1
          className={cn(
            "mb-[3px] text-[22px]/6 font-bold",
            battle.outcome === "defeat" && "text-[#ffb0a2]"
          )}
        >
          {labels[battle.outcome]}
        </h1>
        <p className="text-[17px]/[22px] whitespace-nowrap">
          Kill Count <strong className="font-medium">+{numbers.format(battle.killCount)}</strong>
        </p>
        <p className="text-[15px]/5 whitespace-nowrap text-[#e2ca96]">
          Trade <strong className="font-semibold text-[#ffe4a5]">{trade}%</strong>
        </p>
      </div>
    </article>
  );
}
