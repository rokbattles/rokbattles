"use client";

import { cn } from "cnfast";
import Image from "next/image";

type ReportSkillSlotProps = {
  spriteUrls: string[];
  level?: number;
  alt: string;
  title?: string;
  className?: string;
};

export function ReportSkillSlot({
  spriteUrls,
  level,
  alt,
  title,
  className,
}: ReportSkillSlotProps) {
  return (
    <div
      className={cn(
        "relative h-9 w-9 select-none overflow-hidden rounded-md bg-zinc-600/10 dark:bg-white/5 sm:h-10 sm:w-10",
        className
      )}
      title={title}
    >
      {spriteUrls.length > 0 ? (
        spriteUrls.map((spriteUrl) => (
          <Image
            key={spriteUrl}
            src={spriteUrl}
            alt={alt}
            fill
            sizes="(min-width: 640px) 40px, 36px"
            className="object-contain"
          />
        ))
      ) : (
        <div className="flex h-full w-full items-center justify-center text-[10px] font-semibold text-zinc-300">
          -
        </div>
      )}
      {typeof level === "number" && Number.isFinite(level) ? (
        <span className="absolute bottom-0.5 left-0.5 rounded bg-black/70 px-1 text-[10px] font-semibold leading-4 text-white">
          {level}
        </span>
      ) : null}
    </div>
  );
}
