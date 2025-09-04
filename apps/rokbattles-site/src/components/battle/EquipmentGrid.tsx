import { EquipmentSlot, type EquipmentToken } from "@/components/battle/EquipmentSlot";

type Props = {
  slots: Record<number, EquipmentToken | undefined>;
};

export async function EquipmentGrid({ slots }: Props) {
  return (
    <div className="inline-grid grid-cols-[auto_auto_auto] gap-2">
      <div />
      <EquipmentSlot token={slots[2]} />
      <div />
      <EquipmentSlot token={slots[1]} />
      <EquipmentSlot token={slots[3]} />
      <EquipmentSlot token={slots[4]} />
      <EquipmentSlot token={slots[7]} />
      <EquipmentSlot token={slots[5]} />
      <EquipmentSlot token={slots[8]} />
      <div />
      <EquipmentSlot token={slots[6]} />
      <div />
    </div>
  );
}
