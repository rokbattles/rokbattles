import Image from "next/image";
import { cn } from "@/lib/cn";
import { getCommanderSprites } from "@/lib/commander";

const UNKNOWN_COMMANDER_ICON = "https://cdn.rokbattles.com/game/ui/commander_unknown.png";

type CommanderIconProps = {
  id: number | null | undefined;
  alt: string;
  awakened?: boolean | null;
  className?: string;
  fallback?: boolean;
  sizes?: string;
};

export function CommanderIcon({
  id,
  alt,
  awakened = false,
  className,
  fallback = true,
  sizes = "32px",
}: CommanderIconProps) {
  const spriteUrls = getCommanderSprites(id, awakened === true);
  const fallbackUrl = fallback ? UNKNOWN_COMMANDER_ICON : null;

  if (!spriteUrls?.length && !fallbackUrl) {
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
      {spriteUrls?.length ? (
        spriteUrls.map((spriteUrl) => (
          <Image
            key={spriteUrl}
            alt=""
            className="object-contain"
            fill
            loading="lazy"
            sizes={sizes}
            src={spriteUrl}
            unoptimized
          />
        ))
      ) : (
        <Image
          alt=""
          className="object-cover"
          fill
          loading="lazy"
          sizes={sizes}
          src={fallbackUrl}
          unoptimized
        />
      )}
    </span>
  );
}
