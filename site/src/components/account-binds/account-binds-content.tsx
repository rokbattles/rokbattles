"use client";

import { useExtracted } from "next-intl";
import { useCallback, useState } from "react";
import { BindsClaimSection } from "@/components/account-binds/binds-claim-section";
import { BindsList } from "@/components/account-binds/binds-list";
import { Text } from "@/components/ui/text";
import { useCurrentUser } from "@/hooks/use-current-user";
import { setDefaultBind, unlinkBind } from "@/lib/account-binds/api";
import type { CurrentUser } from "@/lib/types/current-user";

type AccountBindsContentProps = {
  initialUser: CurrentUser;
};

type PendingBindAction = {
  governorId: number;
  action: "set-default" | "unlink";
} | null;

export function AccountBindsContent({ initialUser }: AccountBindsContentProps) {
  const t = useExtracted();
  const { user, refresh } = useCurrentUser({ initialUser });
  const [errorMessage, setErrorMessage] = useState<string | null>(null);
  const [pendingAction, setPendingAction] = useState<PendingBindAction>(null);

  const resolvedUser = user ?? initialUser;
  const claimedBinds = resolvedUser.claimedGovernors ?? [];
  const canClaimMore = claimedBinds.length < 3;

  const handleClaimed = useCallback(async () => {
    await refresh();
    setErrorMessage(null);
  }, [refresh]);

  const handleSetDefault = useCallback(
    async (governorId: number) => {
      setPendingAction({ governorId, action: "set-default" });
      setErrorMessage(null);

      try {
        await setDefaultBind(governorId);
        await refresh();
      } catch (error) {
        const message =
          error instanceof Error
            ? error.message
            : t("Unable to set default bind. Please try again.");
        setErrorMessage(message);
      } finally {
        setPendingAction(null);
      }
    },
    [refresh, t]
  );

  const handleUnlink = useCallback(
    async (governorId: number) => {
      setPendingAction({ governorId, action: "unlink" });
      setErrorMessage(null);

      try {
        await unlinkBind(governorId);
        await refresh();
      } catch (error) {
        const message =
          error instanceof Error ? error.message : t("Unable to unlink bind. Please try again.");
        setErrorMessage(message);
      } finally {
        setPendingAction(null);
      }
    },
    [refresh, t]
  );

  return (
    <div className="mt-8 space-y-8">
      {errorMessage ? <Text className="text-red-600 dark:text-red-400">{errorMessage}</Text> : null}
      <BindsList
        binds={claimedBinds}
        pendingAction={pendingAction}
        onSetDefault={handleSetDefault}
        onUnlink={handleUnlink}
      />
      <BindsClaimSection canClaimMore={canClaimMore} onClaimed={handleClaimed} />
    </div>
  );
}
