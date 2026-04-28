"use client";

import { useExtracted } from "next-intl";
import { Text } from "@/components/ui/text";

export function DuelEmptyState() {
  const t = useExtracted();
  return (
    <Text role="status" aria-live="polite">
      {t("No reports were found for this duel.")}
    </Text>
  );
}
