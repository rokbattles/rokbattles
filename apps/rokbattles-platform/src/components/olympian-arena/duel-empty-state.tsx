"use client";

import { useTranslations } from "next-intl";
import { Text } from "@/components/ui/text";

export function DuelEmptyState() {
  const t = useTranslations("duels");
  return (
    <Text aria-live="polite" role="status">
      {t("states.empty")}
    </Text>
  );
}
