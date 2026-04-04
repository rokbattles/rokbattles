import { cookies } from "next/headers";
import { redirect } from "next/navigation";
import { getExtracted } from "next-intl/server";
import { AccountLootContent } from "@/components/account-loot/account-loot-content";
import { Heading } from "@/components/ui/heading";
import { defaultLocale, isLocale, languageCookieName } from "@/i18n/config";
import { requireCurrentUserWithGovernor } from "@/lib/require-user";

export default async function Page() {
  const user = await requireCurrentUserWithGovernor();
  const t = await getExtracted();
  const governorId = user.claimedGovernors[0]?.governorId;

  if (governorId == null) {
    redirect("/account/settings/governors");
  }

  const cookieStore = await cookies();
  const localeFromCookie = cookieStore.get(languageCookieName)?.value;
  const datasetLocale = isLocale(localeFromCookie) ? localeFromCookie : defaultLocale;

  return (
    <>
      <Heading>{t("My Loot")}</Heading>
      <AccountLootContent datasetLocale={datasetLocale} />
    </>
  );
}
