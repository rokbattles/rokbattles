import { getExtracted } from "next-intl/server";
import { Text } from "@/components/ui/text";

export async function ArkMatchHistoryLoadingState() {
  const t = await getExtracted();
  return <Text className="mt-8">{t("Loading Ark match history...")}</Text>;
}
