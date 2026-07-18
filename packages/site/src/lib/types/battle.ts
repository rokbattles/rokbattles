export type BattleMail = {
  metadata: BattleMetadata;
  sender: BattleSender;
  summary: BattleSummary;
  opponents: readonly BattleOpponent[];
  timeline: BattleTimeline;
};

export type BattleMetadata = {
  mailId: string;
  mailTime: number;
  mailRole: string | null;
  kvk: boolean | null;
};

export type BattleSender = BattlePlayer;

export type BattleOpponent = BattlePlayer & {
  startTick: number;
  endTick: number;
  attack: BattleAttack;
  npc: BattleNpc;
  battleResults: BattleOpponentBattleResults;
  battleEffects?: BattleEffects;
};

export type BattleEffects = {
  sender: readonly BattleStratagem[];
  opponent: readonly BattleStratagem[];
};

export type BattleStratagem = {
  id: number;
  name: string;
  description: string;
  effectivePercentage?: number;
  statistics: readonly BattleStratagemStatistic[];
};

export type BattleStratagemStatistic = {
  key: string;
  value: unknown;
  displayValue?: number;
  unit?: "number" | "percent";
};

export type BattlePlayer = {
  playerId: number;
  playerName: string;
  alliance: BattleAlliance;
  avatarUrl: string | null;
  frameUrl: string | null;
  avatarOverride: boolean;
  trackingKey: string | null;
  rally: boolean | null;
  allianceBuildingId: number | null;
  castle: BattleCastle;
  appUid: number | null;
  commanders: BattleCommanderSet;
  supportSkills: BattleSupportSkills;
  auxiliarySkills: readonly BattleAuxiliarySkill[];
};

export type BattleAlliance = {
  abbreviation: string;
};

export type BattleCastle = {
  x: number;
  y: number;
};

export type BattleCommanderSet = {
  primary: BattleCommander;
  secondary: BattleCommander;
};

export type BattleCommander = {
  id: number | null;
  awakened: boolean | null;
  level: number | null;
  formation: number | null;
  equipment: string | null;
  relics: readonly BattleCommanderRelic[];
  skills: readonly BattleCommanderSkill[];
  armaments: readonly BattleCommanderArmament[];
};

export type BattleCommanderRelic = {
  id: number;
};

export type BattleCommanderSkill = {
  id: number;
  level: number;
};

export type BattleCommanderArmament = {
  affix: string | null;
  buffs: string | null;
};

export type BattleSupportSkills = {
  enable: boolean | null;
  skills: readonly BattleSupportSkill[];
};

export type BattleSupportSkill = {
  heroId: number;
  skillId: number;
  skillLevel: number;
};

export type BattleAuxiliarySkill = {
  heroId: number;
  level: number;
  skillId: number;
};

export type BattleAttack = {
  x: number;
  y: number;
};

export type BattleNpc = {
  type: number | null;
  bType: number | null;
};

export type BattleOpponentBattleResults = {
  sender: BattleDetailedResult;
  opponent: BattleDetailedResult;
};

export type BattleDetailedResult = {
  reinforcementsJoin: number | null;
  reinforcementsLeave: number | null;
  killPoints: number | null;
  acclaim: number | null;
  severelyWounded: number | null;
  slightlyWounded: number | null;
  remaining: number | null;
  dead: number | null;
  heal: number | null;
  troopUnits: number | null;
  troopUnitsMax: number | null;
  power: number | null;
  attackPower: number | null;
  skillPower: number | null;
};

export type BattleSummary = {
  sender: BattleSummarySide;
  opponent: BattleSummarySide;
};

export type BattleSummarySide = {
  killPoints: number | null;
  dead: number | null;
  severelyWounded: number | null;
  slightlyWounded: number | null;
  remaining: number | null;
  troopUnits: number | null;
};

export type BattleTimeline = {
  startTimestamp: number;
  startTick: number;
};
