import { AccountReportsContent } from "@/components/account/account-reports-content";
import { requireCurrentUserWithGovernor } from "@/lib/require-user";

// @next-codemod-ignore Cache Components adoption: this segment temporarily allows blocking.
// Remove this opt-out after verifying the segment passes validation without it.
// See: https://nextjs.org/docs/app/guides/migrating-to-cache-components
export const instant = false;

export default async function Page() {
  await requireCurrentUserWithGovernor();

  return <AccountReportsContent title="Rallies" kind="rally" />;
}
