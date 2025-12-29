import { ReportEquipmentSlot } from "@/components/report/ReportEquipmentSlot";
import { Subheading } from "@/components/ui/Heading";
import type { EquipmentToken } from "@/lib/report/parsers";

type ReportEquipmentSectionProps = {
  tokens: EquipmentToken[];
};

export function ReportEquipmentSection({ tokens }: ReportEquipmentSectionProps) {
  if (tokens.length === 0) {
    return null;
  }

  const slots = tokens.reduce<Record<number, EquipmentToken | undefined>>((acc, token) => {
    acc[token.slot] = token;
    return acc;
  }, {});

  return (
    <div className="space-y-2">
      <Subheading>Equipment</Subheading>
      <div className="flex justify-center">
        <div className="grid grid-cols-[auto_auto_auto] gap-2 justify-items-center">
          <div />
          <ReportEquipmentSlot token={slots[2]} />
          <div />
          <ReportEquipmentSlot token={slots[1]} />
          <ReportEquipmentSlot token={slots[3]} />
          <ReportEquipmentSlot token={slots[4]} />
          <ReportEquipmentSlot token={slots[7]} />
          <ReportEquipmentSlot token={slots[5]} />
          <ReportEquipmentSlot token={slots[8]} />
          <div />
          <ReportEquipmentSlot token={slots[6]} />
          <div />
        </div>
      </div>
    </div>
  );
}
