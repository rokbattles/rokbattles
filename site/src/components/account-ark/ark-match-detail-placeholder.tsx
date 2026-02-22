import { getExtracted } from "next-intl/server";
import { Text } from "@/components/ui/text";

type ArkMatchDetailPlaceholderProps = {
  matchId: string;
};

export async function ArkMatchDetailPlaceholder({ matchId }: ArkMatchDetailPlaceholderProps) {
  const t = await getExtracted();

  return (
    <section className="mt-8 space-y-4">
      <Text>{t("Match ID: {id}", { id: matchId })}</Text>
    </section>
  );
}
