import { Text } from "@/components/ui/text";

export function ReportErrorState({ message }: { message: string }) {
  return (
    <Text role="status" aria-live="polite">
      {message}
    </Text>
  );
}
