import { useExtracted } from "next-intl";
import { CombatLabMessage } from "@/components/combat-lab/combat-lab-message";

export function CombatLabLoading() {
  const t = useExtracted();

  return (
    <CombatLabMessage
      title={t("Loading Combat Lab data")}
      message={t("Fetching the selected commander pairing.")}
    />
  );
}
