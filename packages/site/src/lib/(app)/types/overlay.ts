export type OverlayBranding = "icon_watermark" | "text_watermark";

export type OverlayPreferences = {
  limit: number;
  branding: OverlayBranding;
};

export type BattleCardData = {
  id: string;
  commanders: {
    primaryCommanderId: number;
    secondaryCommanderId: number;
    primaryCommanderAwakened: boolean | null;
    secondaryCommanderAwakened: boolean | null;
  };
  outcome: "victory" | "defeat" | "battle" | "unknown";
  killCount: number;
  tradePercent: number | null;
};

export type OverlayCommander = {
  name: string;
  sprites: string[];
  rarity: string;
};

export type OverlayUrlAction = "rotate" | "disable";
