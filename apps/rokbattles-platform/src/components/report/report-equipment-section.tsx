"use client";

import { useTranslations } from "next-intl";
import { ReportEquipmentSlot } from "@/components/report/report-equipment-slot";
import { Subheading } from "@/components/ui/heading";
import type { EquipmentToken } from "@/lib/report/parsers";

interface ReportEquipmentSectionProps {
  tokens: EquipmentToken[];
}

export function ReportEquipmentSection({
  tokens,
}: ReportEquipmentSectionProps) {
  const t = useTranslations("report");
  const slots = tokens.reduce<Record<number, EquipmentToken | undefined>>(
    (acc, token) => {
      acc[token.slot] = token;
      return acc;
    },
    {}
  );

  return (
    <div className="space-y-2">
      <Subheading>{t("equipment.title")}</Subheading>
      <div className="flex justify-center">
        <div className="grid grid-cols-[auto_auto_auto] justify-items-center gap-2">
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
