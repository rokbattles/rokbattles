import { cn } from "cn";
import Image from "next/image";
import type { ReactElement } from "react";
import { getGameSpriteUrl } from "@/lib/game-sprite";

type ArkAllianceEmblemProps = {
  logo: string | null;
  isBlue: boolean | null;
  large?: boolean;
};

export function ArkAllianceEmblem({
  logo,
  isBlue,
  large = false,
}: ArkAllianceEmblemProps): ReactElement {
  const pattern = logo?.split("_")[2];
  const id = pattern && /^-?\d+$/.test(pattern) ? Number(pattern) : 0;
  const hasPattern = (id >= 1 && id <= 79) || (id >= -4 && id <= -2);

  return (
    <span
      aria-hidden="true"
      className={cn(
        "flex shrink-0 items-center justify-center rounded-md",
        large ? "size-16 p-2" : "size-9 p-1",
        isBlue ? "bg-blue-600" : "bg-red-600"
      )}
    >
      {hasPattern ? (
        <Image
          src={getGameSpriteUrl(`img_AlliFlagPattern${id}.png`)}
          alt=""
          width={140}
          height={140}
          unoptimized
          className="size-full object-contain brightness-0 invert"
        />
      ) : (
        <span className="text-lg font-semibold text-white">◆</span>
      )}
    </span>
  );
}
