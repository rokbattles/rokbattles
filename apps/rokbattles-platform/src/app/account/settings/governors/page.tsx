"use client";

import { ClaimGovernorForm } from "@/components/governors/ClaimGovernorForm";
import { Avatar } from "@/components/ui/Avatar";
import { Subheading } from "@/components/ui/Heading";
import { Text } from "@/components/ui/Text";
import { useCurrentUser } from "@/hooks/useCurrentUser";

export default function Page() {
  const { user, loading, refresh } = useCurrentUser();

  if (loading) {
    return (
      <p className="mt-8 text-sm text-zinc-500 dark:text-zinc-400" role="status" aria-live="polite">
        Loading your account&hellip;
      </p>
    );
  }

  if (!user) {
    return (
      <p className="mt-8 text-sm text-zinc-500 dark:text-zinc-400" role="status" aria-live="polite">
        You must be logged in to view this page.
      </p>
    );
  }

  const claimedGovernors = user.claimedGovernors ?? [];
  const canClaimMore = claimedGovernors.length < 3;

  return (
    <div className="space-y-8 mt-8">
      <section className="space-y-4">
        <Subheading level={3}>Claimed governors</Subheading>
        {claimedGovernors.length === 0 ? (
          <Text>No governors claimed yet.</Text>
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
                      ID {governor.governorId}
                    </p>
                  ) : null}
                </div>
              </li>
            ))}
          </ul>
        )}
      </section>
      <section className="space-y-4">
        <Subheading level={3}>Claim a governor</Subheading>
        <Text>
          Link a governor to your account by entering the Governor ID from Rise of Kingdoms. You can
          claim up to three.
        </Text>
        <ClaimGovernorForm canClaimMore={canClaimMore} onClaimed={refresh} />
      </section>
    </div>
  );
}
