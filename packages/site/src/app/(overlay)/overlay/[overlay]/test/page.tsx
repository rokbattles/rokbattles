import { notFound } from "next/navigation";
import type { JSX } from "react";
import { type BattleCardData, BattleCards } from "@/components/overlay/battle-cards";
import { getOverlayCommanders } from "@/lib/overlay-commanders";
import type { OverlayPreferences } from "@/lib/types/overlay";

const samples: Pick<BattleCardData, "outcome" | "killCount" | "tradePercent">[] = [
  { outcome: "victory", killCount: 28107, tradePercent: 215 },
  { outcome: "defeat", killCount: 8432, tradePercent: 67 },
  { outcome: "battle", killCount: 15680, tradePercent: 104 },
  { outcome: "victory", killCount: 123456, tradePercent: 348 },
  { outcome: "defeat", killCount: 950, tradePercent: 23 },
  { outcome: "battle", killCount: 64205, tradePercent: 100 },
  { outcome: "victory", killCount: 1250000, tradePercent: null },
];

const battles: BattleCardData[] = samples.map((sample, index) => ({
  id: `sample-${index}`,
  commanders: {
    primaryCommanderId: 1,
    secondaryCommanderId: index % 2 === 0 ? 0 : 3,
    primaryCommanderAwakened: false,
    secondaryCommanderAwakened: false,
  },
  ...sample,
}));

export default async function Page({
  params,
}: {
  params: Promise<{ overlay: string }>;
}): Promise<JSX.Element> {
  const { overlay } = await params;

  if (!/^ov_[A-Za-z0-9_-]{43}$/.test(overlay)) {
    notFound();
  }

  const apiUrl = process.env.API_URL || "http://localhost:8001";
  const response = await fetch(`${apiUrl}/v1/overlay/${overlay}/settings`, {
    cache: "no-store",
    signal: AbortSignal.timeout(10_000),
  });

  if (response.status === 404) {
    notFound();
  }

  if (!response.ok) {
    throw new Error("Unable to load overlay settings");
  }

  const { limit, branding }: OverlayPreferences = await response.json();

  return (
    <BattleCards
      battles={battles.slice(0, limit)}
      commanders={getOverlayCommanders()}
      branding={branding}
      status="test"
    />
  );
}
