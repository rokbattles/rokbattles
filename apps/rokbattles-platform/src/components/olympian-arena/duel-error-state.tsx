import { Text } from "@/components/ui/Text";

export function DuelErrorState({ message }: { message: string }) {
  return (
    <Text role="status" aria-live="polite">
      {message}
    </Text>
  );
}
