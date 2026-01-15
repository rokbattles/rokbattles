"use client";

import { LoadoutEquipmentSlot } from "@/components/my-pairings/loadout-equipment-slot";
import type { LoadoutSnapshot } from "@/hooks/usePairings";

type LoadoutEquipmentGridProps = {
  tokens: LoadoutSnapshot["equipment"];
};

export function LoadoutEquipmentGrid({ tokens }: LoadoutEquipmentGridProps) {
  const slots = tokens.reduce<Record<number, LoadoutSnapshot["equipment"][number] | undefined>>(
    (acc, token) => {
      acc[token.slot] = token;
      return acc;
    },
    {}
  );

  return (
    <div className="flex justify-center">
      <div className="grid grid-cols-[auto_auto_auto] gap-2 justify-items-center">
        <div />
        <LoadoutEquipmentSlot token={slots[2]} />
        <div />
        <LoadoutEquipmentSlot token={slots[1]} />
        <LoadoutEquipmentSlot token={slots[3]} />
        <LoadoutEquipmentSlot token={slots[4]} />
        <LoadoutEquipmentSlot token={slots[7]} />
        <LoadoutEquipmentSlot token={slots[5]} />
        <LoadoutEquipmentSlot token={slots[8]} />
        <div />
        <LoadoutEquipmentSlot token={slots[6]} />
        <div />
      </div>
    </div>
  );
}
