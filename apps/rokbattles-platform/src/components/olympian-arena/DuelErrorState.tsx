import { Text } from "@/components/ui/Text";

export function DuelErrorState({ message }: { message: string }) {
  return <Text>We could not load this duel. {message}</Text>;
}
