"use client";

import { useExtracted } from "next-intl";
import { BindListItem } from "@/components/account-binds/bind-list-item";
import { Subheading } from "@/components/ui/heading";
import { Text } from "@/components/ui/text";
import type { ClaimedGovernor } from "@/lib/types/current-user";

type PendingBindAction = {
  governorId: number;
  action: "set-default" | "unlink";
} | null;

type BindsListProps = {
  binds: ClaimedGovernor[];
  pendingAction: PendingBindAction;
  onSetDefault: (governorId: number) => Promise<void>;
  onUnlink: (governorId: number) => Promise<void>;
};

export function BindsList({ binds, pendingAction, onSetDefault, onUnlink }: BindsListProps) {
  const t = useExtracted();

  return (
    <section className="space-y-4">
      <Subheading level={3}>{t("Binds")}</Subheading>
      {binds.length === 0 ? (
        <Text>{t("No binds yet.")}</Text>
      ) : (
        <ul className="divide-y divide-zinc-950/5 rounded border border-zinc-950/10 text-sm dark:divide-white/10 dark:border-white/10">
          {binds.map((bind) => (
            <BindListItem
              key={bind.governorId}
              bind={bind}
              isPending={pendingAction?.governorId === bind.governorId}
              onSetDefault={onSetDefault}
              onUnlink={onUnlink}
            />
          ))}
        </ul>
      )}
    </section>
  );
}
