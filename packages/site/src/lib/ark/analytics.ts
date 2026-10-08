import type { ArkMatchAlliance, ArkMatchRecord } from "@/lib/types/ark";

export function getArkTeams(match: ArkMatchRecord): {
  own: ArkMatchAlliance | undefined;
  opponent: ArkMatchAlliance | undefined;
} {
  const own = match.alliances.find(
    (alliance) => alliance.id != null && alliance.id === match.selfAllianceId
  );
  const opponent = own ? match.alliances.find((alliance) => alliance.id !== own.id) : undefined;

  return { own, opponent };
}

export function getArkOutcome(match: ArkMatchRecord): "win" | "loss" | "draw" | "unknown" {
  const { own, opponent } = getArkTeams(match);

  if (!own || !opponent) {
    return "unknown";
  }

  if (match.winnerAllianceId === own.id) {
    return "win";
  }

  if (opponent.id != null && match.winnerAllianceId === opponent.id) {
    return "loss";
  }

  if (own.score != null && opponent.score != null && own.score === opponent.score) {
    return "draw";
  }

  return "unknown";
}

export function averageArkValue(values: (number | null | undefined)[]): number | null {
  const recorded = values.filter((value): value is number => value != null && value >= 0);

  return recorded.length ? recorded.reduce((sum, value) => sum + value, 0) / recorded.length : null;
}
