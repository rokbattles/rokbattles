import { Text } from "@/components/ui/Text";
import { getEquipmentName } from "@/hooks/useEquipmentName";
import type { AccessoryPairCount } from "@/lib/types/trends";

export function AccessoryPairLabel({ pair }: { pair?: AccessoryPairCount }) {
  if (!pair) {
    return <Text className="text-xs text-zinc-500">No accessory pairs recorded.</Text>;
  }

  const [firstId, secondId] = pair.ids;
  return (
    <div className="text-sm text-zinc-950 dark:text-white">
      {getEquipmentName(firstId) ?? "Unknown"}{" "}
      <span className="text-zinc-600 dark:text-zinc-400">and</span>{" "}
      {getEquipmentName(secondId) ?? "Unknown"}
    </div>
  );
}
