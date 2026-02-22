import { getExtracted } from "next-intl/server";
import { Text } from "@/components/ui/text";

export async function ArkMatchDetailErrorState() {
  const t = await getExtracted();

  return (
    <section className="mt-8 space-y-2">
      <Text>{t("Failed to load Ark match details.")}</Text>
    </section>
  );
}
