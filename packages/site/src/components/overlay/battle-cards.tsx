import { cn } from "cn";
import Image from "next/image";
import type { JSX } from "react";
import { getGameSpriteUrl } from "@/lib/game-sprite";
import type { OverlayCommander } from "@/lib/overlay-commanders";

export type BattleCardData = {
  id: string;
  commanders: {
    primaryCommanderId: number;
    secondaryCommanderId: number;
    primaryCommanderAwakened: boolean | null;
    secondaryCommanderAwakened: boolean | null;
  };
  outcome: "victory" | "defeat" | "battle" | "unknown";
  killCount: number;
  tradePercent: number | null;
};

type PortraitProps = {
  commander: OverlayCommander | undefined;
  awakened: boolean | null;
  secondary?: boolean;
};

type BattleCardProps = {
  battle: BattleCardData;
  commanders: Record<string, OverlayCommander>;
};

type BattleCardsProps = {
  battles: BattleCardData[];
  commanders: Record<string, OverlayCommander>;
  status: string;
};

const numbers = new Intl.NumberFormat("en-US");
const labels = { victory: "Victory", defeat: "Defeat", battle: "Battle", unknown: "Battle" };
const awakenedBackgrounds: Record<string, string> = {
  legendary: "img_icon_HeroProfile_BGMask_Orange.png",
  epic: "img_icon_HeroProfile_BGMask_pink.png",
};

function Portrait({ commander, awakened, secondary = false }: PortraitProps): JSX.Element | null {
  if (!commander) {
    return null;
  }

  const background = awakened ? awakenedBackgrounds[commander.rarity] : undefined;

  return (
    <span
      className={cn(
        "absolute",
        secondary ? "top-[13px] left-[39px] size-12" : "top-0 left-0 size-[62px]"
      )}
      role="img"
      aria-label={commander.name}
    >
      {commander.sprites.map((sprite, index) => {
        const source = index === 0 && background ? background : sprite;

        return (
          <Image
            key={sprite}
            src={getGameSpriteUrl(source)}
            alt=""
            className="object-contain"
            fill
            unoptimized
            sizes="64px"
            preload
          />
        );
      })}
    </span>
  );
}

function BattleCard({ battle, commanders }: BattleCardProps): JSX.Element {
  const trade = battle.tradePercent === null ? "∞" : numbers.format(battle.tradePercent);

  return (
    <article
      data-outcome={battle.outcome}
      className={cn(
        "flex min-h-[94px] animate-battle-arrive items-center rounded-[9px] border-8 border-transparent px-2.5 py-1.5",
        "bg-[rgb(73_52_21/75%)] bg-clip-padding drop-shadow-[0_2px_2px_#0006] motion-reduce:animate-none",
        "[border-image:url('https://cdn.rokbattles.com/game/sprites/img_mailbg4.png')_14_fill_/_14px_/_0_stretch]"
      )}
    >
      <div className="relative h-[62px] w-[94px] shrink-0">
        <Portrait
          commander={commanders[battle.commanders.secondaryCommanderId]}
          awakened={battle.commanders.secondaryCommanderAwakened}
          secondary
        />
        <Portrait
          commander={commanders[battle.commanders.primaryCommanderId]}
          awakened={battle.commanders.primaryCommanderAwakened}
        />
      </div>

      <div className="min-w-0">
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

export function BattleCards({ battles, commanders, status }: BattleCardsProps): JSX.Element {
  return (
    <main
      aria-label="Recent battles"
      data-status={status}
      className="flex w-[390px] max-w-screen flex-col gap-1 p-1 text-[#fff9e9] tabular-nums [font-family:Segoe_UI,Arial,sans-serif]"
    >
      {battles.map((battle) => (
        <BattleCard key={battle.id} battle={battle} commanders={commanders} />
      ))}
    </main>
  );
}
