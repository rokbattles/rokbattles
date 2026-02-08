export interface ExploreOlympianArenaCommanderPair {
  primary: number;
  secondary: number | null;
}

export interface ExploreOlympianArenaRowDb {
  _id: number;
  first_mail_time: number;
  sender_commanders: ExploreOlympianArenaCommanderPair;
  opponent_commanders: ExploreOlympianArenaCommanderPair;
  kill_count: number;
  trade_percentage: number;
  win_streak: number;
}

export interface ExploreOlympianArenaPageDb {
  rows: ExploreOlympianArenaRowDb[];
}

export interface ExploreOlympianArenaRow {
  id: string;
  teamId: number;
  mailTime: number;
  senderCommanders: ExploreOlympianArenaCommanderPair;
  opponentCommanders: ExploreOlympianArenaCommanderPair;
  killCount: number;
  tradePercentage: number;
  winStreak: number;
}

export interface ExploreOlympianArenaPage {
  rows: ExploreOlympianArenaRow[];
  nextAfter: string | null;
  previousBefore: string | null;
}
