import type { BattleCardData } from "@/lib/(app)/types/overlay";

const samples: Pick<BattleCardData, "outcome" | "killCount" | "tradePercent">[] = [
  { outcome: "victory", killCount: 28107, tradePercent: 215 },
  { outcome: "defeat", killCount: 8432, tradePercent: 67 },
  { outcome: "battle", killCount: 15680, tradePercent: 104 },
  { outcome: "victory", killCount: 123456, tradePercent: 348 },
  { outcome: "defeat", killCount: 950, tradePercent: 23 },
  { outcome: "battle", killCount: 64205, tradePercent: 100 },
  { outcome: "victory", killCount: 1250000, tradePercent: null },
];

export const overlaySampleBattles: BattleCardData[] = samples.map((sample, index) => ({
  id: `sample-${index}`,
  commanders: {
    primaryCommanderId: 1,
    secondaryCommanderId: index % 2 === 0 ? 0 : 3,
    primaryCommanderAwakened: false,
    secondaryCommanderAwakened: false,
  },
  ...sample,
}));
