import { MyPairingsContent } from "@/components/my-pairings/my-pairings-content";
import { requireCurrentUserWithGovernor } from "@/lib/require-user";

// @next-codemod-ignore Cache Components adoption: this segment temporarily allows blocking.
// Remove this opt-out after verifying the segment passes validation without it.
// See: https://nextjs.org/docs/app/guides/migrating-to-cache-components
export const instant = false;

export default async function Page() {
  await requireCurrentUserWithGovernor();

  return <MyPairingsContent />;
}
