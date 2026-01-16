import { Text } from "@/components/ui/text";

export function DuelErrorState({ message }: { message: string }) {
  return (
    <Text aria-live="polite" role="status">
      {message}
    </Text>
  );
}
