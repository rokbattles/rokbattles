import "server-only";

import type { Db, Document } from "mongodb";
import type {
  ArkBattleInfoMailDocument,
  ArkBattleResultsMailDocument,
  ArkIndividualResultsMailDocument,
} from "@/lib/types/ark";

const battleResultsProjection: Document = {
  _id: 0,
  "metadata.mail_id": 1,
  "metadata.mail_time": 1,
  "body.win": 1,
  "body.alliance.id": 1,
  "alliances.alliance.id": 1,
  "alliances.alliance.name": 1,
  "alliances.alliance.abbreviation": 1,
  "alliances.score": 1,
  "alliances.members": 1,
  "alliances.members_max": 1,
  "alliances.is_blue": 1,
};

const battleInfoProjection: Document = {
  _id: 0,
  "metadata.mail_id": 1,
  "metadata.mail_time": 1,
  "body.win": 1,
  "body.fights.team": 1,
  "body.fights.time": 1,
  "body.fights.win": 1,
};

const individualResultsProjection: Document = {
  _id: 0,
  "metadata.mail_id": 1,
  "metadata.mail_time": 1,
  "body.team": 1,
  "body.win": 1,
  "overview.player_id": 1,
  "overview.player_name": 1,
  "overview.rank": 1,
  "overview.score": 1,
  "overview.total_results.battles": 1,
  "overview.total_results.kill_points": 1,
  "overview.total_results.severely_wounded": 1,
  "results.total_score": 1,
  "results.win_rate": 1,
  "results.battles_win": 1,
  "results.battles_lose": 1,
  "results.severely_wounded": 1,
  "results.kills": 1,
  "results.kill_score": 1,
  "results.flag_score": 1,
  "results.building_score": 1,
  "results.gather_score": 1,
  "results.units_healed": 1,
  "results.speedups": 1,
  "results.teleports": 1,
  "results.structures": 1,
  "pairings.primary_commander.id": 1,
  "pairings.secondary_commander.id": 1,
  "pairings.battles": 1,
  "pairings.battles_win": 1,
  "pairings.kill_count": 1,
  "pairings.kill_points": 1,
  "pairings.severely_wounded": 1,
};

export async function fetchArkBattleResultsMails(
  db: Db,
  options: { mailReceiver: string; limit: number }
): Promise<ArkBattleResultsMailDocument[]> {
  const { mailReceiver, limit } = options;

  return db
    .collection<ArkBattleResultsMailDocument>("mails_alliance_aoobattleresults")
    .find(
      {
        "metadata.mail_receiver": mailReceiver,
      },
      { projection: battleResultsProjection }
    )
    .sort({ "metadata.mail_time": -1 })
    .limit(limit)
    .toArray();
}

export async function fetchArkBattleResultsMailById(
  db: Db,
  options: { mailReceiver: string; mailId: string }
): Promise<ArkBattleResultsMailDocument | null> {
  const { mailReceiver, mailId } = options;

  return db.collection<ArkBattleResultsMailDocument>("mails_alliance_aoobattleresults").findOne(
    {
      $and: [{ "metadata.mail_receiver": mailReceiver }, { "metadata.mail_id": mailId }],
    },
    { projection: battleResultsProjection }
  );
}

export async function fetchArkBattleInfoMails(
  db: Db,
  options: { mailReceiver: string; timeMatch: Document }
): Promise<ArkBattleInfoMailDocument[]> {
  const { mailReceiver, timeMatch } = options;

  return db
    .collection<ArkBattleInfoMailDocument>("mails_alliance_aoobattleinfo")
    .find(
      {
        $and: [{ "metadata.mail_receiver": mailReceiver }, timeMatch],
      },
      { projection: battleInfoProjection }
    )
    .toArray();
}

export async function fetchArkIndividualResultsMails(
  db: Db,
  options: { mailReceiver: string; timeMatch: Document }
): Promise<ArkIndividualResultsMailDocument[]> {
  const { mailReceiver, timeMatch } = options;

  return db
    .collection<ArkIndividualResultsMailDocument>("mails_alliance_aooindividualresults")
    .find(
      {
        $and: [{ "metadata.mail_receiver": mailReceiver }, timeMatch],
      },
      { projection: individualResultsProjection }
    )
    .toArray();
}
