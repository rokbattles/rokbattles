export interface DuelReportPayload {
  metadata: {
    email_id: string;
    email_time: number;
    email_receiver: string;
    server_id: number;
  };
  sender: DuelParticipantInfo;
  opponent: DuelParticipantInfo;
  results: DuelResults;
}

export interface DuelParticipantInfo {
  player_id: number;
  player_name: string;
  kingdom: number;
  alliance: string;
  duel_id: number;
  avatar_url: string;
  frame_url: string;
  commanders: {
    primary: DuelCommanderInfo;
    secondary: DuelCommanderInfo;
  };
  buffs: DuelBuffEntry[];
}

export interface DuelCommanderInfo {
  id: number;
  level: number;
  star: number;
  awakened: boolean;
  skills: DuelSkillInfo[];
}

export interface DuelSkillInfo {
  id: number;
  level: number;
  order: number;
}

export interface DuelBuffEntry {
  id: number;
  value: number;
}

export interface DuelResults {
  kill_points: number;
  sev_wounded: number;
  wounded: number;
  dead: number;
  heal: number;
  units: number;
  power: number;
  win: boolean;
  opponent_kill_points: number;
  opponent_sev_wounded: number;
  opponent_wounded: number;
  opponent_dead: number;
  opponent_heal: number;
  opponent_units: number;
  opponent_power: number;
  opponent_win: boolean;
}
