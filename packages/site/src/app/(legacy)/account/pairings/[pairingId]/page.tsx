import { notFound } from "next/navigation";
import { PairingDetailContent } from "@/components/my-pairings/pairing-detail-content";
import { requireCurrentUserWithGovernor } from "@/lib/require-user";

export default async function Page({ params }: PageProps<"/account/pairings/[pairingId]">) {
  await requireCurrentUserWithGovernor();

  const { pairingId } = await params;
  if (!/^[1-9]\d*-\d+$/.test(pairingId)) notFound();

  const [primary, secondary] = pairingId.split("-").map(Number);
  if (!Number.isSafeInteger(primary) || !Number.isSafeInteger(secondary)) notFound();

  return <PairingDetailContent primary={primary} secondary={secondary} />;
}
