export interface RawReportMetadata {
  email_id?: string;
  email_time?: number;
  email_type?: string;
  email_receiver?: string;
  email_box?: string;
  email_role?: string | null;
  is_kvk?: number;
  attack_id?: string;
  start_date?: number;
  end_date?: number;
  pos_x?: number;
  pos_y?: number;
  players?: unknown;
}

export interface RawCommanderInfo {
  id?: number;
  level?: number;
  skills?: string;
}

export interface RawParticipantInfo {
  player_id?: number;
  player_name?: string;
  alliance_tag?: string;
  avatar_url?: string;
  castle_x?: number;
  castle_y?: number;
  is_rally?: boolean | null;
  npc_type?: number | null;
  npc_btype?: number | null;
  primary_commander?: RawCommanderInfo;
  secondary_commander?: RawCommanderInfo;
  equipment?: string;
  formation?: number;
  armament_buffs?: string;
  inscriptions?: string;
}

export interface RawBattleResults {
  power?: number;
  init_max?: number;
  max?: number;
  healing?: number;
  death?: number;
  severely_wounded?: number;
  wounded?: number;
  remaining?: number;
  watchtower?: number;
  watchtower_max?: number;
  kill_score?: number;
  enemy_power?: number;
  enemy_init_max?: number;
  enemy_max?: number;
  enemy_healing?: number;
  enemy_death?: number;
  enemy_severely_wounded?: number;
  enemy_wounded?: number;
  enemy_remaining?: number;
  enemy_watchtower?: number;
  enemy_watchtower_max?: number;
  enemy_kill_score?: number;
}

export interface RawReportPayload {
  metadata?: RawReportMetadata;
  self?: RawParticipantInfo;
  enemy?: RawParticipantInfo;
  battle_results?: RawBattleResults;
}
