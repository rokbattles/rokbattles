import { cn } from "cn";
import Image from "next/image";
import { getGameSpriteUrl } from "@/lib/game-sprite";

const ITEM_FRAME_URL = getGameSpriteUrl("item_icon_frame_state1.png");

type LootIconProps = {
  spriteUrls?: string[];
  className?: string;
};

export function LootIcon({ spriteUrls, className }: LootIconProps) {
  if (!spriteUrls?.length) {
    return null;
  }

  const hasItemFrame = spriteUrls.includes(ITEM_FRAME_URL);

  return (
    <span
      aria-hidden="true"
      className={cn("relative inline-grid h-8 w-8 shrink-0 *:col-start-1 *:row-start-1", className)}
    >
      {spriteUrls.map((spriteUrl) => (
        <Image
          key={spriteUrl}
          alt=""
          className={cn(
            spriteUrl === ITEM_FRAME_URL ? "scale-y-[1.014] object-fill" : "object-contain",
            hasItemFrame && spriteUrl !== ITEM_FRAME_URL && "-translate-y-[3.214%] scale-[0.814]"
          )}
          fill
          loading="lazy"
          sizes="32px"
          src={spriteUrl}
          unoptimized
        />
      ))}
    </span>
  );
}
