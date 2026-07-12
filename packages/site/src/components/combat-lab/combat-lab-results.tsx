import { useExtracted } from "next-intl";
import { CombatLabSummarySection } from "@/components/combat-lab/combat-lab-summary-section";
import { DrastcSection } from "@/components/combat-lab/drastc-section";
import { Subheading } from "@/components/ui/heading";
import { Text } from "@/components/ui/text";
import type { CombatLabPairingDocument } from "@/lib/combat-lab/api";
import { formatRefreshedAt } from "@/lib/combat-lab/format";

type CombatLabResultsProps = {
  item: CombatLabPairingDocument;
  primaryName: string;
  secondaryName: string;
};

export function CombatLabResults({ item, primaryName, secondaryName }: CombatLabResultsProps) {
  const t = useExtracted();
  const refreshedAt = formatRefreshedAt(item.refreshedAt);

  return (
    <div className="space-y-8">
      <div className="space-y-1">
        <Subheading>{`${primaryName} / ${secondaryName}`}</Subheading>
        <Text>{t("Last updated: {date}", { date: refreshedAt })}</Text>
      </div>
      {item.drastc ? <DrastcSection score={item.drastc} /> : null}
      <CombatLabSummarySection item={item} />
    </div>
  );
}
