"use client";

import { useTranslations } from "next-intl";
import { ClaimGovernorForm } from "@/components/governors/claim-governor-form";
import { Avatar } from "@/components/ui/avatar";
import { Subheading } from "@/components/ui/heading";
import { Text } from "@/components/ui/text";
import { useCurrentUser } from "@/hooks/use-current-user";
import type { CurrentUser } from "@/lib/types/current-user";

interface AccountGovernorsContentProps {
  initialUser: CurrentUser;
}

export function AccountGovernorsContent({
  initialUser,
}: AccountGovernorsContentProps) {
  const t = useTranslations("account");
  const tCommon = useTranslations("common");
  const { user, refresh } = useCurrentUser({ initialUser });
  const resolvedUser = user ?? initialUser;

  const claimedGovernors = resolvedUser.claimedGovernors ?? [];
  const canClaimMore = claimedGovernors.length < 3;

  return (
    <div className="mt-8 space-y-8">
      <section className="space-y-4">
        <Subheading level={3}>{t("governors.claimedTitle")}</Subheading>
        {claimedGovernors.length === 0 ? (
          <Text>{t("governors.noneClaimed")}</Text>
        ) : (
          <ul className="divide-y divide-zinc-950/5 rounded border border-zinc-950/10 text-sm dark:divide-white/10 dark:border-white/10">
            {claimedGovernors.map((governor) => (
              <li
                className="flex items-center gap-3 px-4 py-3"
                key={governor.governorId}
              >
                {governor.governorAvatar ? (
                  <Avatar
                    className="size-10"
                    square
                    src={governor.governorAvatar}
                  />
                ) : (
                  <Avatar className="size-10" initials="G" square />
                )}
                <div className="min-w-0">
                  <p className="truncate font-medium text-zinc-950 dark:text-white">
                    {governor.governorName ?? governor.governorId.toString()}
                  </p>
                  {governor.governorName ? (
                    <p className="text-xs text-zinc-500 dark:text-zinc-400">
                      {tCommon("labels.id", { id: governor.governorId })}
                    </p>
                  ) : null}
                </div>
              </li>
            ))}
          </ul>
        )}
      </section>
      <section className="space-y-4">
        <Subheading level={3}>{t("governors.claimTitle")}</Subheading>
        <Text>{t("governors.claimDescription")}</Text>
        <ClaimGovernorForm canClaimMore={canClaimMore} onClaimed={refresh} />
      </section>
    </div>
  );
}
