import { Text } from "@/components/ui/Text";

export function ReportErrorState({ message }: { message: string }) {
  return <Text>We could not load this report. {message}</Text>;
}
