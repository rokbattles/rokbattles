import { Text } from "@/components/ui/Text";

export function ReportEmptyState() {
  return (
    <Text role="status" aria-live="polite">
      No battles were found for this hash.
    </Text>
  );
}
