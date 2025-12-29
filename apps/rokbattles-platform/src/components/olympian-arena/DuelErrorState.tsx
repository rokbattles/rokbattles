import { Text } from "@/components/ui/Text";

export function DuelErrorState({ message }: { message: string }) {
  return (
    <Text role="status" aria-live="polite">
      We could not load this duel. {message}
    </Text>
  );
}
