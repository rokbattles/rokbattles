import { cn } from "cn";
import type { ReactNode } from "react";
import { Text } from "@/components/ui/text";

type SettingsActionRowProps = {
  label: string;
  description: string;
  inset?: boolean;
  children: ReactNode;
};

export function SettingsActionRow({
  label,
  description,
  inset = false,
  children,
}: SettingsActionRowProps) {
  return (
    <div className={cn("flex flex-wrap items-center justify-between gap-4 py-3", inset && "px-3")}>
      <div className="space-y-1">
        <p className="text-sm">{label}</p>
        <Text>{description}</Text>
      </div>
      {children}
    </div>
  );
}
