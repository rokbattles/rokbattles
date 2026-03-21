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
};

export type BattlePlayer = {
  playerId: number;
  playerName: string;
  alliance: BattleAlliance;
  avatarUrl: string | null;
  frameUrl: string | null;
  trackingKey: string | null;
  rally: boolean | null;
  allianceBuildingId: number | null;
  castle: BattleCastle;
  appUid: number | null;
  commanders: BattleCommanderSet;
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
  level: number | null;
  formation: number | null;
  equipment: string | null;
  skills: readonly BattleCommanderSkill[];
  armaments: readonly BattleCommanderArmament[];
};

export type BattleCommanderSkill = {
  id: number;
  level: number;
};

export type BattleCommanderArmament = {
  affix: string | null;
  buffs: string | null;
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
