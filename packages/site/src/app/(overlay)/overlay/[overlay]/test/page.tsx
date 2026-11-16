import { notFound } from "next/navigation";
import type { JSX } from "react";
import { BattleCards } from "@/components/(app)/overlay/battle-cards";
import { getOverlayCommanders } from "@/lib/(app)/overlay-commanders";
import { overlaySampleBattles } from "@/lib/(app)/overlay-samples";
import type { OverlayPreferences } from "@/lib/(app)/types/overlay";

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
      battles={overlaySampleBattles.slice(0, limit)}
      commanders={getOverlayCommanders()}
      branding={branding}
      status="test"
    />
  );
}
