import { roman } from "@/lib/roman";

type Props = {
  tier: number;
};

export async function EquipmentTierBadge({ tier }: Props) {
  return (
    <div className="absolute -bottom-1 left-1/2 -translate-x-1/2 rounded-full bg-zinc-900/90 px-1.5 py-0.5 text-[10px] font-bold text-white ring-1 ring-white/15">
      {roman(tier)}
    </div>
  );
}
