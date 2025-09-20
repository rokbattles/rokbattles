export interface ReportEntry {
  self_commander_id?: number;
  self_secondary_commander_id?: number;
  enemy_commander_id?: number;
  enemy_secondary_commander_id?: number;
  start_date?: number;
}

export interface ReportItem {
  hash: string;
  entries: ReportEntry[];
}

export interface ReportsResponse {
  items: ReportItem[];
  next_cursor?: string;
  count?: number;
}

export interface ReportsWithNamesResponse extends ReportsResponse {
  names?: Record<string, string | undefined>;
  error?: string;
}

export interface ReportMetadata {
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

export interface CommanderInfo {
  id?: number;
  level?: number;
  skills?: string;
}

export interface ParticipantInfo {
  player_id?: number;
  player_name?: string;
  alliance_tag?: string;
  avatar_url?: string;
  castle_x?: number;
  castle_y?: number;
  is_rally?: boolean | null;
  npc_type?: number | null;
  npc_btype?: number | null;
  primary_commander?: CommanderInfo;
  secondary_commander?: CommanderInfo;
  kingdom_id?: number;
  tracking_key?: string;
  equipment?: string;
  formation?: number;
  armament_buffs?: string;
  inscriptions?: string;
}

export interface BattleResults {
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

export interface BattleResultsTimelineEntry {
  start_date?: number;
  end_time?: number | null;
  death?: number;
  severely_wounded?: number;
  wounded?: number;
  remaining?: number;
  kill_score?: number;
  enemy_death?: number;
  enemy_severely_wounded?: number;
  enemy_wounded?: number;
  enemy_remaining?: number;
  enemy_kill_score?: number;
}

export interface BattleResultsSummary {
  total?: BattleResults;
  timeline?: BattleResultsTimelineEntry[];
}

export interface SingleReportInner {
  metadata?: ReportMetadata;
  self?: ParticipantInfo;
  enemy?: ParticipantInfo;
  battle_results?: BattleResults;
}

export interface SingleReportItem {
  hash: string;
  report: SingleReportInner;
  start_date?: number;
}

export interface SingleReportResponse {
  parent_hash: string;
  items: SingleReportItem[];
  next_cursor?: string;
  count?: number;
  battle_results?: BattleResultsSummary;
}
