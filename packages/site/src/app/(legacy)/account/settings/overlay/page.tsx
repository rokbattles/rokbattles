import { notFound } from "next/navigation";
import { OverlaySettings } from "@/components/overlay/overlay-settings";
import { hasOverlayAccess } from "@/lib/overlay-access";
import { requireCurrentUserWithGovernor } from "@/lib/require-user";

export default async function Page() {
  const user = await requireCurrentUserWithGovernor();
  if (!hasOverlayAccess(user.discordId)) notFound();
  return <OverlaySettings />;
}
