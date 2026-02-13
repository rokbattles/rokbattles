export type BattleMail = {
  metadata: BattleMetadata;
  sender: BattleSender;
  summary: BattleSummary;
  opponents: readonly BattleOpponent[];
  timeline: BattleTimeline;
};

export type BattleMetadata = {
  mail_id: string;
  mail_time: number;
  mail_receiver: string;
  server_id: number;
  mail_role: string;
};

export type BattleSender = BattlePlayer;

export type BattleOpponent = BattlePlayer & {
  attack: BattleAttack;
  start_tick: number;
  end_tick: number;
  npc: BattleNpc;
  battle_results: BattleOpponentBattleResults;
};

export type BattlePlayer = {
  player_id: number;
  player_name: string;
  kingdom_id: number | null;
  alliance: BattleAlliance;
  alliance_building_id: number | null;
  castle: BattleCastle;
  tracking_key: string;
  camp_id: number | null;
  rally: boolean | null;
  structure_id: number | null;
  commanders: BattleCommanderSet;
  app_id: number | null;
  app_uid: number | null;
  avatar_url: string | null;
  frame_url: string | null;
  supreme_strife: BattleSupremeStrife;
  participants: readonly BattleParticipant[];
};

export type BattleAlliance = {
  id: number;
  name: string;
  abbreviation: string;
};

export type BattleCastle = {
  x: number;
  y: number;
  level: number;
  watchtower: number | null;
};

export type BattleSupremeStrife = {
  battle_id: string | null;
  team_id: number | null;
  round: number | null;
};

export type BattleCommanderSet = {
  primary: BattleCommander;
  secondary: BattleCommander;
};

export type BattleCommander = {
  id: number | null;
  level: number | null;
  formation: number | null;
  awakened: boolean | null;
  star_level: number | null;
  equipment: string | null;
  skills: readonly BattleCommanderSkill[] | null;
  relics: readonly BattleCommanderRelic[] | null;
  armaments: readonly BattleCommanderArmament[] | null;
};

export type BattleCommanderSkill = {
  id: number;
  level: number;
};

export type BattleCommanderRelic = {
  id: number;
  level: number;
};

export type BattleCommanderArmament = {
  id: number;
  affix: string;
  buffs: string;
};

export type BattleParticipant = {
  participant_id: number;
  player_id: number;
  player_name: string;
  alliance: {
    abbreviation: string;
  };
  commanders: {
    primary: BattleParticipantCommander;
    secondary: BattleParticipantCommander;
  };
};

export type BattleParticipantCommander = {
  id: number | null;
  level: number | null;
};

export type BattleAttack = {
  id: string;
  x: number;
  y: number;
};

export type BattleNpc = {
  type: number | null;
  b_type: number | null;
  experience: number | null;
  loot: readonly BattleLoot[] | null;
};

export type BattleLoot = {
  type: number;
  sub_type: number;
  value: number;
};

export type BattleOpponentBattleResults = {
  sender: BattleDetailedResult;
  opponent: BattleDetailedResult;
};

export type BattleDetailedResult = {
  reinforcements_join: number | null;
  reinforcements_leave: number | null;
  kill_points: number | null;
  acclaim: number | null;
  severely_wounded: number | null;
  slightly_wounded: number | null;
  remaining: number | null;
  dead: number | null;
  heal: number | null;
  troop_units: number | null;
  troop_units_max: number | null;
  watchtower_max: number | null;
  watchtower: number | null;
  power: number | null;
  attack_power: number | null;
  skill_power: number | null;
  merits: number | null;
  death_reduction: number | null;
  severe_wound_reduction: number | null;
};

export type BattleSummary = {
  sender: BattleSummarySide;
  opponent: BattleSummarySide;
};

export type BattleSummarySide = {
  kill_points: number | null;
  dead: number | null;
  severely_wounded: number | null;
  slightly_wounded: number | null;
  remaining: number | null;
  troop_units: number | null;
};

export type BattleTimeline = {
  start_timestamp: number;
  end_timestamp: number;
  start_tick: number;
  sampling: readonly BattleTimelineSample[];
  events: readonly BattleTimelineEvent[];
};

export type BattleTimelineSample = {
  tick: number;
  count: number;
};

export type BattleTimelineEvent = {
  tick: number;
  type: number;
  event_id: number | null;
  player_id: number;
  player_name: string;
  count: number | null;
  avatar_url: string | null;
  frame_url: string | null;
  commanders: {
    primary: BattleParticipantCommander;
    secondary: BattleParticipantCommander;
  };
};
