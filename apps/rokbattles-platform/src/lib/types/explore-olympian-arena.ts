export interface ExploreOlympianArenaCommanderPair {
  primary: number | null;
  secondary: number | null;
}

export interface ExploreOlympianArenaRowDb {
  _id: number;
  first_mail_time: number | null;
  sender_commanders: ExploreOlympianArenaCommanderPair;
  opponent_commanders: ExploreOlympianArenaCommanderPair;
  trade_percentage: number;
  win_streak: number;
}

export interface ExploreOlympianArenaPageDb {
  rows: ExploreOlympianArenaRowDb[];
  total: number;
}

export interface ExploreOlympianArenaRow {
  id: string;
  teamId: number;
  mailTime: number | null;
  senderCommanders: ExploreOlympianArenaCommanderPair;
  opponentCommanders: ExploreOlympianArenaCommanderPair;
  tradePercentage: number;
  winStreak: number;
}

export interface ExploreOlympianArenaPage {
  rows: ExploreOlympianArenaRow[];
  total: number;
}
