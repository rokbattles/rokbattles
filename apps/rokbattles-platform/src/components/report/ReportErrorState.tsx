import { Text } from "@/components/ui/Text";

export function ReportErrorState({ message }: { message: string }) {
  return (
    <Text role="status" aria-live="polite">
      We could not load this report. {message}
    </Text>
  );
}
