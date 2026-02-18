"use client";

import { useExtracted } from "next-intl";
import { Text } from "@/components/ui/text";

export function ReportEmptyState() {
  const t = useExtracted();
  return (
    <Text role="status" aria-live="polite">
      {t("No battles were found for this hash.")}
    </Text>
  );
}
