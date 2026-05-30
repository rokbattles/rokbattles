import type { ArkMatchAlliance, ArkMatchRecord } from "@/lib/types/ark";

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

export function getArkSethAlliance(record: ArkMatchRecord): ArkMatchAlliance | null {
  const match = record.alliances.find((alliance) => alliance.isBlue !== true);
  return match ?? null;
}

export function getArkIsetAlliance(record: ArkMatchRecord): ArkMatchAlliance | null {
  const match = record.alliances.find((alliance) => alliance.isBlue === true);
  return match ?? null;
}

export function getArkWinnerSide(record: ArkMatchRecord): "seth" | "iset" | null {
  if (record.winnerAllianceId == null) {
    return null;
  }

  const sethAlliance = getArkSethAlliance(record);
  if (sethAlliance?.id != null && sethAlliance.id === record.winnerAllianceId) {
    return "seth";
  }

  const isetAlliance = getArkIsetAlliance(record);
  if (isetAlliance?.id != null && isetAlliance.id === record.winnerAllianceId) {
    return "iset";
  }

  return null;
}
