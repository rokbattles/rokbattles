import { Fragment } from "react";
import { ReportInscriptionBadge } from "@/components/report/ReportInscriptionBadge";
import {
  DescriptionDetails,
  DescriptionList,
  DescriptionTerm,
} from "@/components/ui/DescriptionList";
import { Subheading } from "@/components/ui/Heading";
import { getArmamentInfo } from "@/hooks/useArmamentName";
import type { ArmamentBuff } from "@/lib/report/parsers";

type ReportArmamentSectionProps = {
  buffs: ArmamentBuff[];
  inscriptions: number[];
};

export function ReportArmamentSection({ buffs, inscriptions }: ReportArmamentSectionProps) {
  if (buffs.length === 0 && inscriptions.length === 0) {
    return null;
  }

  return (
    <div className="space-y-2">
      <Subheading>Armament Info</Subheading>
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
              <DescriptionTerm className="pt-1! pb-1! border-none!">
                {getArmamentInfo(buff.id ?? null)?.name ?? `Armament ${buff.id}`}
              </DescriptionTerm>
              <DescriptionDetails className="pb-1! pt-1! border-none! sm:text-right tabular-nums">
                {(Number(buff.value ?? 0) * 100).toFixed(2)}%
              </DescriptionDetails>
            </Fragment>
          ))}
        </DescriptionList>
      ) : null}
    </div>
  );
}
