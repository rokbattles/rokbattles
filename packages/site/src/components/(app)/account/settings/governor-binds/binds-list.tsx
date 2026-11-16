"use client";

import { BindListItem } from "@/components/(app)/account/settings/governor-binds/bind-list-item";
import { SettingsSection } from "@/components/(app)/account/settings/settings-section";
import { Text } from "@/components/ui/text";
import type { ClaimedGovernor } from "@/lib/types/current-user";

type BindsListProps = {
  binds: ClaimedGovernor[];
  pendingGovernorId: number | null;
  onSetDefault: (governorId: number) => Promise<void>;
  onUnlink: (governorId: number) => Promise<void>;
};

export function BindsList({ binds, pendingGovernorId, onSetDefault, onUnlink }: BindsListProps) {
  return (
    <SettingsSection
      title="Governor Binds"
      description="These are the governors linked to your ROK Battles account. You can manage your binds or link another governor."
    >
      {binds.length === 0 ? (
        <Text>No binds yet.</Text>
      ) : (
        <ul className="divide-y rounded border text-sm divide-white/10 border-white/10">
          {binds.map((bind) => (
            <BindListItem
              key={bind.governorId}
              bind={bind}
              isPending={pendingGovernorId === bind.governorId}
              onSetDefault={onSetDefault}
              onUnlink={onUnlink}
            />
          ))}
        </ul>
      )}
    </SettingsSection>
  );
}
