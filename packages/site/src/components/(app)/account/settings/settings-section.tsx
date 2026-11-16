import type { ReactNode } from "react";
import { Subheading } from "@/components/ui/heading";
import { Text } from "@/components/ui/text";

type SettingsSectionProps = {
  title: string;
  description?: ReactNode;
  children?: ReactNode;
};

export function SettingsSection({ title, description, children }: SettingsSectionProps) {
  return (
    <section className="space-y-4">
      <div className="space-y-1">
        <Subheading>{title}</Subheading>
        {description ? <Text>{description}</Text> : null}
      </div>
      {children}
    </section>
  );
}
