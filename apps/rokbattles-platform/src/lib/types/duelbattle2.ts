export type DuelBattle2Mail = {
  metadata: DuelBattle2Metadata;
  sender: DuelBattle2Player;
  opponent: DuelBattle2Player;
  battle_results: DuelBattle2BattleResults;
};

export type DuelBattle2MailDocument = DuelBattle2Mail & {
  _id?: string;
};

export type DuelBattle2Metadata = {
  mail_id: string;
  mail_time: number;
  mail_receiver: string;
  server_id: number;
};

export type DuelBattle2Player = {
  player_id: number;
  player_name: string;
  avatar_url: string | null;
  frame_url: string | null;
  alliance: {
    abbreviation: string;
  };
  duel: {
    team_id: number;
  };
  primary_commander: DuelBattle2Commander;
  secondary_commander: DuelBattle2Commander;
  buffs: readonly DuelBattle2Buff[];
};

export type DuelBattle2Commander = {
  id: number;
  level: number;
  star_level: number;
  awakened: boolean;
  skills: readonly DuelBattle2Skill[];
};

export type DuelBattle2Skill = {
  id: number;
  level: number;
};

export type DuelBattle2Buff = {
  id: number;
  value: number;
};

export type DuelBattle2BattleResults = {
  sender: DuelBattle2BattleResult;
  opponent: DuelBattle2BattleResult;
};

export type DuelBattle2BattleResult = {
  win: boolean;
  kill_points: number;
  power: number;
  units: number;
  slightly_wounded: number;
  severely_wounded: number;
  dead: number;
  heal: number;
};
