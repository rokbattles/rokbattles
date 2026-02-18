"use client";

import { useExtracted } from "next-intl";
import { ClaimGovernorForm } from "@/components/governors/claim-governor-form";
import { Avatar } from "@/components/ui/avatar";
import { Subheading } from "@/components/ui/heading";
import { Text } from "@/components/ui/text";
import { useCurrentUser } from "@/hooks/use-current-user";
import type { CurrentUser } from "@/lib/types/current-user";

type AccountGovernorsContentProps = {
  initialUser: CurrentUser;
};

export function AccountGovernorsContent({ initialUser }: AccountGovernorsContentProps) {
  const t = useExtracted();
  const { user, refresh } = useCurrentUser({ initialUser });
  const resolvedUser = user ?? initialUser;

  const claimedGovernors = resolvedUser.claimedGovernors ?? [];
  const canClaimMore = claimedGovernors.length < 3;

  return (
    <div className="space-y-8 mt-8">
      <section className="space-y-4">
        <Subheading level={3}>{t("Claimed governors")}</Subheading>
        {claimedGovernors.length === 0 ? (
          <Text>{t("No governors claimed yet.")}</Text>
        ) : (
          <ul className="divide-y divide-zinc-950/5 rounded border border-zinc-950/10 text-sm dark:divide-white/10 dark:border-white/10">
            {claimedGovernors.map((governor) => (
              <li key={governor.governorId} className="flex items-center gap-3 px-4 py-3">
                {governor.governorAvatar ? (
                  <Avatar src={governor.governorAvatar} className="size-10" square />
                ) : (
                  <Avatar initials="G" className="size-10" square />
                )}
                <div className="min-w-0">
                  <p className="truncate font-medium text-zinc-950 dark:text-white">
                    {governor.governorName ?? governor.governorId.toString()}
                  </p>
                  {governor.governorName ? (
                    <p className="text-xs text-zinc-500 dark:text-zinc-400">
                      {t("ID {id}", { id: governor.governorId.toString() })}
                    </p>
                  ) : null}
                </div>
              </li>
            ))}
          </ul>
        )}
      </section>
      <section className="space-y-4">
        <Subheading level={3}>{t("Claim a governor")}</Subheading>
        <Text>
          {t(
            "Link a governor to your account by entering the Governor ID from Rise of Kingdoms. You can claim up to three."
          )}
        </Text>
        <ClaimGovernorForm canClaimMore={canClaimMore} onClaimed={refresh} />
      </section>
    </div>
  );
}
