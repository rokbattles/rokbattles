import { cookies } from "next/headers";
import { redirect } from "next/navigation";
import { PersonalLootContent } from "@/components/account-loot/personal-loot-content";
import { defaultLocale, isLocale, languageCookieName } from "@/i18n/config";
import { requireCurrentUserWithGovernor } from "@/lib/require-user";

export default async function Page() {
  const user = await requireCurrentUserWithGovernor();
  if (user.claimedGovernors[0]?.governorId == null) {
    redirect("/account/settings/governors");
  }

  const localeFromCookie = (await cookies()).get(languageCookieName)?.value;
  const datasetLocale = isLocale(localeFromCookie) ? localeFromCookie : defaultLocale;
  return (
    <PersonalLootContent
      active="karuak-ceremony"
      endpoint="karuak-ceremony"
      datasetLocale={datasetLocale}
    />
  );
}
