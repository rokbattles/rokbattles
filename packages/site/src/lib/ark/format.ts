import type { ArkMatchAlliance } from "@/lib/types/ark";

export function formatArkAllianceLabel(
  alliance: ArkMatchAlliance | null | undefined,
  unknownLabel: string
): string {
  if (!alliance) {
    return unknownLabel;
  }

  const abbreviation = alliance.abbreviation?.trim();
  const name = alliance.name?.trim();

  if (abbreviation && name) {
    return `[${abbreviation}] ${name}`;
  }

  if (name) {
    return name;
  }

  if (abbreviation) {
    return abbreviation;
  }

  if (alliance.id != null) {
    return `ID ${alliance.id}`;
  }

  return unknownLabel;
}
