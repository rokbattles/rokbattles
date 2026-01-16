import { Text } from "@/components/ui/text";

export function ReportErrorState({ message }: { message: string }) {
  return (
    <Text aria-live="polite" role="status">
      {message}
    </Text>
  );
}
