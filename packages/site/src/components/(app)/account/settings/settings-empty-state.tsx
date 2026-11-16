import type { ReactNode } from "react";
import { Text } from "@/components/ui/text";

type SettingsEmptyStateProps = {
  children: ReactNode;
  action?: ReactNode;
};

export function SettingsEmptyState({ children, action }: SettingsEmptyStateProps) {
  return (
    <div className="flex flex-col items-center gap-4 rounded-lg border border-dashed border-white/20 px-6 py-8 text-center">
      <Text>{children}</Text>
      {action}
    </div>
  );
}
