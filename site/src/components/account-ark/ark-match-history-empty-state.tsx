"use client";

import { useExtracted } from "next-intl";
import { Text } from "@/components/ui/text";

export function ArkMatchHistoryEmptyState() {
  const t = useExtracted();

  return (
    <section className="mt-8 space-y-2">
      <Text>{t("No Ark match history found.")}</Text>
    </section>
  );
}
