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
  app_uid?: string;
  player_name?: string;
  alliance_tag?: string;
  avatar_url?: string;
  frame_url?: string;
  castle_x?: number;
  castle_y?: number;
  is_rally?: boolean | null;
  alliance_building?: number | null;
  npc_type?: number | null;
  npc_btype?: number | null;
  primary_commander?: RawCommanderInfo;
  secondary_commander?: RawCommanderInfo;
  equipment?: string;
  equipment_2?: string;
  formation?: number;
  armament_buffs?: string;
  inscriptions?: string;
}

export interface RawOverview {
  kill_score?: number | null;
  severely_wounded?: number | null;
  max?: number | null;
  wounded?: number | null;
  remaining?: number | null;
  death?: number | null;
  enemy_kill_score?: number | null;
  enemy_severely_wounded?: number | null;
  enemy_max?: number | null;
  enemy_wounded?: number | null;
  enemy_remaining?: number | null;
  enemy_death?: number | null;
}

export interface RawBattleResults {
  power?: number;
  acclaim?: number;
  reinforcements_join?: number;
  reinforcements_retreat?: number;
  skill_power?: number;
  attack_power?: number;
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
  enemy_acclaim?: number;
  enemy_reinforcements_join?: number;
  enemy_reinforcements_retreat?: number;
  enemy_skill_power?: number;
  enemy_attack_power?: number;
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
  overview?: RawOverview;
  battle_results?: RawBattleResults;
}
