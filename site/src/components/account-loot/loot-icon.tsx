import Image from "next/image";
import { cn } from "@/lib/cn";

type LootIconProps = {
  spriteUrls?: string[];
  className?: string;
};

export function LootIcon({ spriteUrls, className }: LootIconProps) {
  if (!spriteUrls?.length) {
    return null;
  }

  return (
    <span
      aria-hidden="true"
      className={cn("relative inline-grid h-8 w-8 shrink-0 *:col-start-1 *:row-start-1", className)}
    >
      {spriteUrls.map((spriteUrl) => (
        <Image
          key={spriteUrl}
          alt=""
          className="object-contain"
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
