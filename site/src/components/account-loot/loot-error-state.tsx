"use client";

import { useExtracted } from "next-intl";
import { Text } from "@/components/ui/text";

export function LootErrorState() {
  const t = useExtracted();
  return <Text>{t("Failed to load loot.")}</Text>;
}
