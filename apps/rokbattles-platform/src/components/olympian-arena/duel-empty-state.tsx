"use client";

import { useTranslations } from "next-intl";
import { Text } from "@/components/ui/text";

export function DuelEmptyState() {
  const t = useTranslations("duels");
  return (
    <Text role="status" aria-live="polite">
      {t("states.empty")}
    </Text>
  );
}
