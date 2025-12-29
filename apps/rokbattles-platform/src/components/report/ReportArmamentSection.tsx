import { ReportInscriptionBadge } from "@/components/report/ReportInscriptionBadge";
import { Subheading } from "@/components/ui/Heading";
import { Text } from "@/components/ui/Text";
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
        <div className="space-y-1.5">
          {buffs.map((buff) => (
            <Text key={buff.id} className="flex items-center justify-between">
              <span>{getArmamentInfo(buff.id ?? null)?.name ?? `Armament ${buff.id}`}</span>
              <span className="font-mono text-zinc-900 dark:text-white">
                {(Number(buff.value ?? 0) * 100).toFixed(2)}%
              </span>
            </Text>
          ))}
        </div>
      ) : null}
    </div>
  );
}
