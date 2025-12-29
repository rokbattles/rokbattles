import { Text } from "@/components/ui/Text";

export function DuelEmptyState() {
  return (
    <Text role="status" aria-live="polite">
      No reports were found for this duel.
    </Text>
  );
}
