export type DuelBattle2DetailResponse = {
  items: DuelBattle2DetailItem[];
};

export type DuelBattle2DetailItem = {
  metadata: DuelBattle2Metadata;
  sender: DuelBattle2Player;
  opponent: DuelBattle2Player;
  battleResults: DuelBattle2BattleResults;
};

export type DuelBattle2Metadata = {
  mailId: string;
  mailTime: number;
};

export type DuelBattle2Player = {
  playerId: number;
  playerName: string;
  avatarUrl: string | null;
  frameUrl: string | null;
  alliance: {
    abbreviation: string;
  };
  primaryCommander: DuelBattle2Commander;
  secondaryCommander: DuelBattle2Commander;
  buffs: readonly DuelBattle2Buff[];
};

export type DuelBattle2Commander = {
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
  killPoints: number;
  power: number;
  units: number;
  slightlyWounded: number;
  severelyWounded: number;
  dead: number;
  heal: number;
};
