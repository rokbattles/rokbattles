"use client";

import { useExtracted } from "next-intl";
import { use } from "react";
import { CombatLabMessage } from "@/components/combat-lab/combat-lab-message";
import { CombatLabResults } from "@/components/combat-lab/combat-lab-results";
import { loadCombatLabPairingResult } from "@/lib/combat-lab/api";

type CombatLabPairingResultsProps = {
  primaryCommanderId: number;
  primaryName: string;
  secondaryCommanderId: number;
  secondaryName: string;
};

export function CombatLabPairingResults({
  primaryCommanderId,
  primaryName,
  secondaryCommanderId,
  secondaryName,
}: CombatLabPairingResultsProps) {
  const t = useExtracted();
  const result = use(
    loadCombatLabPairingResult({
      primaryCommanderId,
      secondaryCommanderId,
    })
  );

  if (result.status === "error") {
    return (
      <CombatLabMessage
        title={t("Combat Lab data is unavailable")}
        message={result.error || t("Try another pairing.")}
      />
    );
  }

  return (
    <CombatLabResults item={result.item} primaryName={primaryName} secondaryName={secondaryName} />
  );
}
