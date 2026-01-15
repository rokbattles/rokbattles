"use client";

import { useTranslations } from "next-intl";
import { Text } from "@/components/ui/Text";
import { getEquipmentName } from "@/hooks/use-equipment-name";
import type { AccessoryPairCount } from "@/lib/types/trends";

export function AccessoryPairLabel({ pair }: { pair?: AccessoryPairCount }) {
  const t = useTranslations("trends");
  const tCommon = useTranslations("common");
  if (!pair) {
    return <Text className="text-xs text-zinc-500">{t("accessories.emptyPairs")}</Text>;
  }

  const [firstId, secondId] = pair.ids;
  return (
    <div className="text-sm text-zinc-950 dark:text-white">
      {getEquipmentName(firstId) ?? tCommon("labels.unknown")}{" "}
      <span className="text-zinc-600 dark:text-zinc-400">{tCommon("labels.and")}</span>{" "}
      {getEquipmentName(secondId) ?? tCommon("labels.unknown")}
    </div>
  );
}
