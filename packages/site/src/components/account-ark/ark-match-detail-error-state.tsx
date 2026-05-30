"use client";

import { useExtracted } from "next-intl";
import { Text } from "@/components/ui/text";

export function ArkMatchDetailErrorState() {
  const t = useExtracted();

  return (
    <section className="mt-8 space-y-2">
      <Text>{t("Failed to load Ark match details.")}</Text>
    </section>
  );
}
