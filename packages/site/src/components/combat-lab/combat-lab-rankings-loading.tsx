import { CombatLabMessage } from "@/components/combat-lab/combat-lab-message";

export function CombatLabRankingsLoading() {
  return (
    <CombatLabMessage
      title="Loading Combat Lab rankings"
      message="Fetching the latest DRASTC scores."
    />
  );
}
