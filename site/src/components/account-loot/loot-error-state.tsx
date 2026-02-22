import { getExtracted } from "next-intl/server";
import { Text } from "@/components/ui/text";

export async function LootErrorState() {
  const t = await getExtracted();
  return <Text>{t("Failed to load loot.")}</Text>;
}
