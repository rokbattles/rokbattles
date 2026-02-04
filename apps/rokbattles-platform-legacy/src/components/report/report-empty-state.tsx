"use client";

import { useTranslations } from "next-intl";
import { Text } from "@/components/ui/text";

export function ReportEmptyState() {
  const t = useTranslations("report");
  return (
    <Text role="status" aria-live="polite">
      {t("states.empty")}
    </Text>
  );
}
