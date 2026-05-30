"use client";

import { useExtracted } from "next-intl";
import { Text } from "@/components/ui/text";

export function ResourcesErrorState() {
  const t = useExtracted();
  return <Text>{t("Failed to load resources.")}</Text>;
}
