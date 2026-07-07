import { Subheading } from "@/components/ui/heading";
import { Text } from "@/components/ui/text";

type CombatLabMessageProps = {
  title: string;
  message: string;
};

export function CombatLabMessage({ title, message }: CombatLabMessageProps) {
  return (
    <section className="space-y-2 border-zinc-200/60 border-b pb-4 dark:border-white/10">
      <Subheading>{title}</Subheading>
      <Text className="mt-2">{message}</Text>
    </section>
  );
}
