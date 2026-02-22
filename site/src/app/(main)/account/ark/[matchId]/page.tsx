import { getExtracted } from "next-intl/server";
import { ArkMatchDetailPlaceholder } from "@/components/account-ark/ark-match-detail-placeholder";
import { Heading } from "@/components/ui/heading";
import { requireCurrentUserWithGovernor } from "@/lib/require-user";

export default async function Page({ params }: PageProps<"/account/ark/[matchId]">) {
  await requireCurrentUserWithGovernor();
  const t = await getExtracted();
  const { matchId } = await params;

  return (
    <>
      <Heading>{t("Ark Match")}</Heading>
      <ArkMatchDetailPlaceholder matchId={matchId} />
    </>
  );
}
