import { getCommanderSprites } from "@/lib/commander";
import type { BattlePlayer } from "@/lib/types/battle";
import type { ReportByIdResponse } from "@/lib/types/report";

export type ReportEmbedSide = {
  name: string;
  /** Sprite layers for the primary commander, then the secondary one when there is one. */
  commanders: string[][];
  dead: number;
  severelyWounded: number;
};

export type ReportEmbed = {
  /** "Sender vs Opponent", used as the title of the link preview. */
  title: string;
  description: string;
  sender: ReportEmbedSide;
  opponent: ReportEmbedSide;
  /** Microseconds since the epoch, as the API stores it. */
  mailTime: number;
  kvk: boolean;
};

const numberFormat = new Intl.NumberFormat("en-US");

export function formatCount(value: number): string {
  return numberFormat.format(value);
}

function commandersOf(player: BattlePlayer | undefined): string[][] {
  return (["primary", "secondary"] as const)
    .map((which) => {
      const commander = player?.commanders?.[which];
      return getCommanderSprites(commander?.id, commander?.awakened === true) ?? [];
    })
    .filter((sprites) => sprites.length > 0);
}

function sideOf(
  player: BattlePlayer | undefined,
  totals: { dead: number | null; severelyWounded: number | null } | undefined
): ReportEmbedSide {
  const name = player?.playerName?.trim();
  return {
    name: name && name.length > 0 ? name : "Unknown",
    commanders: commandersOf(player),
    dead: totals?.dead ?? 0,
    severelyWounded: totals?.severelyWounded ?? 0,
  };
}

/** Loads the parts of a battle report that the link preview shows. Returns null when it cannot. */
export async function getReportEmbed(id: string): Promise<ReportEmbed | null> {
  const reportId = id.trim();
  if (reportId.length === 0) {
    return null;
  }

  const api = process.env.API_URL || "http://localhost:8001";

  let payload: ReportByIdResponse;
  try {
    const response = await fetch(`${api}/v1/report/battle/${encodeURIComponent(reportId)}`, {
      next: { revalidate: 3600 },
    });
    if (!response.ok) {
      return null;
    }
    payload = (await response.json()) as ReportByIdResponse;
  } catch {
    return null;
  }

  const mail = payload.mail;
  if (!mail?.sender) {
    return null;
  }

  // The opponent side shows the first opponent. The totals cover all of them.
  const sender = sideOf(mail.sender, mail.summary?.sender);
  const opponent = sideOf(mail.opponents?.[0], mail.summary?.opponent);

  return {
    title: `${sender.name} vs ${opponent.name}`,
    description: `Dead ${formatCount(sender.dead)} vs ${formatCount(opponent.dead)} · Severely wounded ${formatCount(sender.severelyWounded)} vs ${formatCount(opponent.severelyWounded)}`,
    sender,
    opponent,
    mailTime: mail.metadata?.mailTime ?? 0,
    kvk: mail.metadata?.kvk === true,
  };
}
