import { cn } from "cn";
import Image from "next/image";
import type { JSX } from "react";
import type { OverlayCommander } from "@/lib/(app)/types/overlay";
import { getGameSpriteUrl } from "@/lib/game-sprite";

type CommanderPortraitProps = {
  commander: OverlayCommander | undefined;
  awakened: boolean | null;
  secondary?: boolean;
};

const awakenedBackgrounds: Record<string, string> = {
  legendary: "img_icon_HeroProfile_BGMask_Orange.png",
  epic: "img_icon_HeroProfile_BGMask_pink.png",
};

export function CommanderPortrait({
  commander,
  awakened,
  secondary = false,
}: CommanderPortraitProps): JSX.Element | null {
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
