import { getExtracted } from "next-intl/server";
import { Text } from "@/components/ui/text";

export async function ArkMatchDetailNotFoundState() {
  const t = await getExtracted();

  return (
    <section className="mt-8 space-y-2">
      <Text>{t("Ark match not found.")}</Text>
    </section>
  );
}
