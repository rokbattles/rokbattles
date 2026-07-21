import { CombatLabLayout } from "@/components/combat-lab/combat-lab-layout";
import { CombatLabRankingsContent } from "@/components/combat-lab/combat-lab-rankings-content";

export default function Page() {
  return (
    <CombatLabLayout active="rankings">
      <CombatLabRankingsContent />
    </CombatLabLayout>
  );
}
