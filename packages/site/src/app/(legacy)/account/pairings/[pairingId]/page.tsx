import { notFound } from "next/navigation";
import { PairingDetailContent } from "@/components/my-pairings/pairing-detail-content";
import { requireCurrentUserWithGovernor } from "@/lib/require-user";

// @next-codemod-ignore Cache Components adoption: this segment temporarily allows blocking.
// Remove this opt-out after verifying the segment passes validation without it.
// See: https://nextjs.org/docs/app/guides/migrating-to-cache-components
export const instant = false;

export default async function Page({ params }: PageProps<"/account/pairings/[pairingId]">) {
  await requireCurrentUserWithGovernor();

  const { pairingId } = await params;
  if (!/^[1-9]\d*-\d+$/.test(pairingId)) notFound();

  const [primary, secondary] = pairingId.split("-").map(Number);
  if (!Number.isSafeInteger(primary) || !Number.isSafeInteger(secondary)) notFound();

  return <PairingDetailContent primary={primary} secondary={secondary} />;
}
