"use client";

import { useExtracted } from "next-intl";
import { BindActionsDropdown } from "@/components/account-binds/bind-actions-dropdown";
import { Avatar } from "@/components/ui/avatar";
import { Badge } from "@/components/ui/badge";
import type { ClaimedGovernor } from "@/lib/types/current-user";

type BindListItemProps = {
  bind: ClaimedGovernor;
  isPending: boolean;
  onSetDefault: (governorId: number) => Promise<void>;
  onUnlink: (governorId: number) => Promise<void>;
};

export function BindListItem({ bind, isPending, onSetDefault, onUnlink }: BindListItemProps) {
  const t = useExtracted();

  return (
    <li className="flex items-center gap-3 px-4 py-3">
      {bind.governorAvatar ? (
        <Avatar src={bind.governorAvatar} className="size-10" square />
      ) : (
        <Avatar initials="G" className="size-10" square />
      )}

      <div className="min-w-0">
        <div className="flex items-center gap-2">
          <p className="truncate font-medium text-zinc-950 dark:text-white">
            {bind.governorName ?? bind.governorId.toString()}
          </p>
          {bind.default ? <Badge color="emerald">{t("Default")}</Badge> : null}
        </div>
        <p className="text-xs text-zinc-500 dark:text-zinc-400">{bind.governorId.toString()}</p>
      </div>

      <div className="ml-auto shrink-0">
        <BindActionsDropdown
          bind={bind}
          disabled={isPending}
          onSetDefault={onSetDefault}
          onUnlink={onUnlink}
        />
      </div>
    </li>
  );
}
