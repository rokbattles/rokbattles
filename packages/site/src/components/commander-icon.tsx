import { cn } from "cn";
import Image from "next/image";
import { getCommanderSprites, isPrimeCommander } from "@/lib/commander";
import { getGameSpriteUrl } from "@/lib/game-sprite";

type CommanderIconProps = {
  id: number | null | undefined;
  alt: string;
  awakened?: boolean | null;
  skinId?: number | null;
  className?: string;
  loading?: "eager" | "lazy";
  sizes?: string;
};

export function CommanderIcon({
  id,
  alt,
  awakened = false,
  skinId,
  className,
  loading = "lazy",
  sizes = "32px",
}: CommanderIconProps) {
  const spriteUrls = getCommanderSprites(id, awakened === true, skinId);

  if (!spriteUrls?.length) {
    return null;
  }

  return (
    <span
      aria-label={alt}
      className={cn(
        "relative inline-grid size-8 shrink-0 align-middle *:col-start-1 *:row-start-1",
        className
      )}
      role="img"
    >
      {spriteUrls.map((spriteUrl) => (
        <Image
          key={spriteUrl}
          alt=""
          className="object-contain"
          fill
          loading={loading}
          sizes={sizes}
          src={spriteUrl}
          unoptimized
        />
      ))}
      {isPrimeCommander(id) ? (
        <span
          className="pointer-events-none absolute"
          style={{
            left: `${((130 - 75 - 20) / 260) * 100}%`,
            top: `${((130 + 48.7 - 20) / 260) * 100}%`,
            width: `${(40 / 260) * 100}%`,
            height: `${(40 / 260) * 100}%`,
          }}
        >
          <Image
            alt=""
            fill
            loading={loading}
            sizes={sizes}
            src={getGameSpriteUrl("img_HeroSystemSpIcon.png")}
            unoptimized
          />
        </span>
      ) : null}
    </span>
  );
}
