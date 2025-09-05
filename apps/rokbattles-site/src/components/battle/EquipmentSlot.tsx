import Image from "next/image";
import { EquipmentSpecialTalentBadge } from "@/components/battle/EquipmentSpecialTalentBadge";
import { EquipmentTierBadge } from "@/components/battle/EquipmentTierBadge";

export type EquipmentToken = {
  slot: number;
  id: number;
  craft: number;
  attr?: number;
};

type Props = {
  token?: EquipmentToken;
};

function deriveTierAndST(attr?: number): { tier?: number; isST: boolean } {
  if (!Number.isFinite(attr) || attr === undefined) return { isST: false };
  const n = Number(attr);
  const isST = n >= 10;
  const base = isST ? n % 10 : n;
  const tier = Math.max(0, Math.min(5, base));
  return { tier, isST };
}

export async function EquipmentSlot({ token }: Props) {
  const id = token?.id;
  const { tier, isST } = deriveTierAndST(token?.attr);

  return (
    <div className="relative h-14 w-14 sm:h-16 sm:w-16 select-none">
      <div className="h-full w-full overflow-hidden rounded-lg ring-1 ring-white/10 bg-zinc-900/60 shadow-sm">
        {id ? (
          <Image
            src={`/lilith/images/equipment/${id}.png`}
            alt={`equipment-${id}`}
            width={64}
            height={64}
            className="h-full w-full object-contain"
            draggable={false}
          />
        ) : (
          <div className="flex h-full w-full items-center justify-center text-[10px] font-semibold text-zinc-300">
            &mdash;
          </div>
        )}
      </div>

      {typeof tier === "number" && tier > 0 && <EquipmentTierBadge tier={tier} />}
      {isST && <EquipmentSpecialTalentBadge />}
    </div>
  );
}
