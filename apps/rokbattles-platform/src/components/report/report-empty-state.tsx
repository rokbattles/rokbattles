"use client";

import { useTranslations } from "next-intl";
import { Text } from "@/components/ui/text";

export function ReportEmptyState() {
  const t = useTranslations("report");
  return (
    <Text aria-live="polite" role="status">
      {t("states.empty")}
    </Text>
  );
}
