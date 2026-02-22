import { cookies } from "next/headers";
import { redirect } from "next/navigation";
import { getExtracted } from "next-intl/server";
import { AccountLootContent } from "@/components/account-loot/account-loot-content";
import { LootErrorState } from "@/components/account-loot/loot-error-state";
import { Heading } from "@/components/ui/heading";
import { getGovernorLootData } from "@/data/loot/query";
import { defaultLocale, isLocale, languageCookieName } from "@/i18n/config";
import { parseLootSearchParams } from "@/lib/loot/search-params";
import { requireCurrentUserWithGovernor } from "@/lib/require-user";

export default async function Page({ searchParams }: PageProps<"/account/loot">) {
  const user = await requireCurrentUserWithGovernor();
  const t = await getExtracted();
  const resolvedSearchParams = (await searchParams) ?? {};
  const parsed = parseLootSearchParams(resolvedSearchParams);
  const governorId = user.claimedGovernors[0]?.governorId;

  if (governorId == null) {
    redirect("/account/settings/governors");
  }

  const cookieStore = await cookies();
  const localeFromCookie = cookieStore.get(languageCookieName)?.value;
  const datasetLocale = isLocale(localeFromCookie) ? localeFromCookie : defaultLocale;

  try {
    const data = await getGovernorLootData({
      governorId,
      startParam: parsed.startParam,
      endParam: parsed.endParam,
      yearParam: parsed.yearParam,
    });

    return (
      <>
        <Heading>{t("My Loot")}</Heading>
        <AccountLootContent
          data={data}
          selectedCategory={parsed.category}
          datasetLocale={datasetLocale}
        />
      </>
    );
  } catch (error) {
    console.error("Failed to load loot data", error);

    return (
      <>
        <Heading>{t("My Loot")}</Heading>
        <LootErrorState />
      </>
    );
  }
}
