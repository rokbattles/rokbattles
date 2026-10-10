import { notFound } from "next/navigation";
import { BattleOverlay } from "@/components/overlay/battle-overlay";
import { getOverlayCommanders } from "@/lib/overlay-commanders";

export default async function Page({ params }: { params: Promise<{ overlay: string }> }) {
  const { overlay } = await params;

  if (!/^ov_[A-Za-z0-9_-]{43}$/.test(overlay)) {
    notFound();
  }

  return <BattleOverlay token={overlay} commanders={getOverlayCommanders()} />;
}
