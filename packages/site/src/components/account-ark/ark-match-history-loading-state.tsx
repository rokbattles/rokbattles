"use client";

import { useExtracted } from "next-intl";
import { Text } from "@/components/ui/text";

export function ArkMatchHistoryLoadingState() {
  const t = useExtracted();

  return <Text className="mt-8">{t("Loading Ark match history...")}</Text>;
}
