"use client";

import { Fragment } from "react";
import {
  DescriptionDetails,
  DescriptionList,
  DescriptionTerm,
} from "@/components/ui/description-list";
import { Subheading } from "@/components/ui/heading";
import { getArmamentInfo } from "@/hooks/use-armament-name";
import type { ArmamentBuff } from "@/lib/report/parsers";

type ReportArmamentSectionProps = {
  buffs: ArmamentBuff[];
};

export function ReportArmamentSection({ buffs }: ReportArmamentSectionProps) {
  if (buffs.length === 0) {
    return null;
  }

  return (
    <div className="space-y-2">
      <Subheading>Armament Info</Subheading>
      <DescriptionList>
        {buffs.map((buff) => (
          <Fragment key={buff.id}>
            <DescriptionTerm className="pt-1! pb-1! border-none!">
              {getArmamentInfo(buff.id ?? null)?.name ?? `Armament ${(buff.id ?? "?").toString()}`}
            </DescriptionTerm>
            <DescriptionDetails className="pb-1! pt-1! border-none! sm:text-right tabular-nums">
              {(Number(buff.value ?? 0) * 100).toFixed(2)}%
            </DescriptionDetails>
          </Fragment>
        ))}
      </DescriptionList>
    </div>
  );
}
