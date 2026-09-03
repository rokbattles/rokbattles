import { BUILDING_SPRITE_CROPS, BUILDING_SPRITES } from "@/lib/territory/assets";
import type { BuildingKind } from "@/lib/territory/types";

type BuildingIconProps = {
  className: string;
  kind: BuildingKind;
};

export function BuildingIcon({ kind, className }: BuildingIconProps) {
  const crop = BUILDING_SPRITE_CROPS[kind];
  return (
    <svg
      aria-hidden="true"
      className={`${className} shrink-0 overflow-hidden`}
      data-slot="sprite"
      focusable="false"
      viewBox={`${crop.x} ${crop.y} ${crop.width} ${crop.height}`}
    >
      <image height="240" href={BUILDING_SPRITES[kind]} width="240" />
    </svg>
  );
}
