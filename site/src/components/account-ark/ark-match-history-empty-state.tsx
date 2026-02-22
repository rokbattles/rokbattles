import { getExtracted } from "next-intl/server";
import { Text } from "@/components/ui/text";

export async function ArkMatchHistoryEmptyState() {
  const t = await getExtracted();

  return (
    <section className="mt-8 space-y-2">
      <Text>{t("No Ark match history found for this governor yet.")}</Text>
    </section>
  );
}
