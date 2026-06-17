import Image from "next/image";
import { cn } from "@/lib/cn";
import { getCommanderSprites } from "@/lib/commander";

type CommanderIconProps = {
  id: number | null | undefined;
  alt: string;
  awakened?: boolean | null;
  className?: string;
  sizes?: string;
};

export function CommanderIcon({
  id,
  alt,
  awakened = false,
  className,
  sizes = "32px",
}: CommanderIconProps) {
  const spriteUrls = getCommanderSprites(id, awakened === true);

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
          loading="lazy"
          sizes={sizes}
          src={spriteUrl}
          unoptimized
        />
      ))}
    </span>
  );
}
