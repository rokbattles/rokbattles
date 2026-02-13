export type BarCanyonKillBossMail = {
  metadata: BarCanyonKillBossMetadata;
  npc: BarCanyonKillBossNpc;
  participants: readonly BarCanyonKillBossParticipant[];
};

export type BarCanyonKillBossMetadata = {
  mail_id: string;
  mail_time: number;
  mail_receiver: string;
  server_id: number;
};

export type BarCanyonKillBossNpc = {
  type: number;
  level: number;
  location: BarCanyonKillBossLocation;
};

export type BarCanyonKillBossLocation = {
  x: number;
  y: number;
};

export type BarCanyonKillBossParticipant = {
  player_id: number;
  player_name: string;
  avatar_url: string | null;
  frame_url: string | null;
  damage_rate: number;
  loot: readonly BarCanyonKillBossLoot[];
};

export type BarCanyonKillBossLoot = {
  type: number;
  sub_type: number;
  value: number;
};
