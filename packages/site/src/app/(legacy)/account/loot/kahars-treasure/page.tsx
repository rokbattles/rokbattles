import { cookies } from "next/headers";
import { redirect } from "next/navigation";
import { PersonalLootContent } from "@/components/account-loot/personal-loot-content";
import { defaultLocale, isLocale, languageCookieName } from "@/i18n/config";
import { requireCurrentUserWithGovernor } from "@/lib/require-user";

export default async function Page() {
  const user = await requireCurrentUserWithGovernor();
  const governorId = user.claimedGovernors[0]?.governorId;

  if (governorId == null) {
    redirect("/account/settings/governors");
  }

  const cookieStore = await cookies();
  const localeFromCookie = cookieStore.get(languageCookieName)?.value;
  const datasetLocale = isLocale(localeFromCookie) ? localeFromCookie : defaultLocale;

  return (
    <PersonalLootContent
      active="kahars-treasure"
      endpoint="kahars-treasure"
      datasetLocale={datasetLocale}
    />
  );
}
