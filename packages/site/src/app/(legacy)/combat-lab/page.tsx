import { CombatLabPage } from "@/components/combat-lab/combat-lab-page";

export default function Page(props: {
  searchParams: Promise<{ primary?: string; secondary?: string }>;
}) {
  return <CombatLabPage {...props} season="soc" />;
}
