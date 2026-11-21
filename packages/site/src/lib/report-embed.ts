import { getCommanderName, getCommanderSprites } from "@/lib/commander";
import type { BattleMail, BattlePlayer } from "@/lib/types/battle";
import type { ReportByIdResponse } from "@/lib/types/report";

export type ReportEmbedCommander = {
  name: string | null;
  sprites: string[];
};

export type ReportEmbedSide = {
  name: string;
  alliance: string | null;
  /** The primary commander first, then the secondary one when there is one. */
  commanders: ReportEmbedCommander[];
  dead: number;
  severelyWounded: number;
};

export type ReportEmbed = {
  /** "Sender vs Opponent", used as the title of the link preview. */
  title: string;
  description: string;
  sender: ReportEmbedSide;
  opponent: ReportEmbedSide;
  /** How many opponents the battle had. The opponent side shows the first one's name and commanders. */
  opponentCount: number;
  mailTime: number;
  kvk: boolean;
};

const numberFormat = new Intl.NumberFormat("en-US");

export function formatCount(value: number): string {
  return numberFormat.format(value);
}

function playerLabel(player: BattlePlayer | undefined, fallback: string) {
  const name = player?.playerName?.trim();
  return name && name.length > 0 ? name : fallback;
}

function commanderOf(
  player: BattlePlayer | undefined,
  which: "primary" | "secondary"
): ReportEmbedCommander | null {
  const commander = player?.commanders?.[which];
  const sprites = getCommanderSprites(commander?.id, commander?.awakened === true) ?? [];
  if (sprites.length === 0) {
    return null;
  }
  return { name: getCommanderName(commander?.id) ?? null, sprites };
}

function sideOf(
  player: BattlePlayer | undefined,
  fallback: string,
  totals: BattleMail["summary"]["sender"] | undefined
): ReportEmbedSide {
  const commanders = [commanderOf(player, "primary"), commanderOf(player, "secondary")].filter(
    (commander): commander is ReportEmbedCommander => commander !== null
  );
  const alliance = player?.alliance?.abbreviation?.trim();
  return {
    name: playerLabel(player, fallback),
    alliance: alliance && alliance.length > 0 ? alliance : null,
    commanders,
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

  const opponent = mail.opponents?.[0];
  const sender = sideOf(mail.sender, "Unknown", mail.summary?.sender);
  const enemy = sideOf(opponent, "Unknown", mail.summary?.opponent);

  return {
    title: `${sender.name} vs ${enemy.name}`,
    description: `Dead ${formatCount(sender.dead)} vs ${formatCount(enemy.dead)} · Severely wounded ${formatCount(sender.severelyWounded)} vs ${formatCount(enemy.severelyWounded)}`,
    sender,
    opponent: enemy,
    opponentCount: mail.opponents?.length ?? 0,
    mailTime: mail.metadata?.mailTime ?? 0,
    kvk: mail.metadata?.kvk === true,
  };
}
