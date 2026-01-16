"use client";

import { useTranslations } from "next-intl";
import { Fragment } from "react";
import { ReportInscriptionBadge } from "@/components/report/report-inscription-badge";
import {
  DescriptionDetails,
  DescriptionList,
  DescriptionTerm,
} from "@/components/ui/description-list";
import { Subheading } from "@/components/ui/heading";
import { getArmamentInfo } from "@/hooks/use-armament-name";
import type { ArmamentBuff } from "@/lib/report/parsers";

interface ReportArmamentSectionProps {
  buffs: ArmamentBuff[];
  inscriptions: number[];
}

export function ReportArmamentSection({
  buffs,
  inscriptions,
}: ReportArmamentSectionProps) {
  const t = useTranslations("report");
  if (buffs.length === 0 && inscriptions.length === 0) {
    return null;
  }

  return (
    <div className="space-y-2">
      <Subheading>{t("armament.title")}</Subheading>
      {inscriptions.length > 0 ? (
        <div className="flex flex-wrap gap-1.5">
          {inscriptions.map((id) => (
            <div key={id}>
              <ReportInscriptionBadge id={id} />
            </div>
          ))}
        </div>
      ) : null}
      {buffs.length > 0 ? (
        <DescriptionList>
          {buffs.map((buff) => (
            <Fragment key={buff.id}>
              <DescriptionTerm className="border-none! pt-1! pb-1!">
                {getArmamentInfo(buff.id ?? null)?.name ??
                  t("armament.fallback", { id: buff.id })}
              </DescriptionTerm>
              <DescriptionDetails className="border-none! pt-1! pb-1! tabular-nums sm:text-right">
                {(Number(buff.value ?? 0) * 100).toFixed(2)}%
              </DescriptionDetails>
            </Fragment>
          ))}
        </DescriptionList>
      ) : null}
    </div>
  );
}
