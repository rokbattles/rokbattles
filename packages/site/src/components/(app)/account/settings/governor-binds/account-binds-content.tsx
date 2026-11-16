"use client";

import { useCallback, useState } from "react";
import { BindsClaimForm } from "@/components/(app)/account/settings/governor-binds/binds-claim-form";
import { BindsList } from "@/components/(app)/account/settings/governor-binds/binds-list";
import { SettingsSection } from "@/components/(app)/account/settings/settings-section";
import { Text } from "@/components/ui/text";
import { useCurrentUser } from "@/hooks/use-current-user";
import { MAX_GOVERNOR_BINDS, setDefaultBind, unlinkBind } from "@/lib/(app)/governor-binds";
import type { CurrentUser } from "@/lib/types/current-user";

type AccountBindsContentProps = {
  initialUser: CurrentUser;
};

type BindAction = "set-default" | "unlink";

export function AccountBindsContent({ initialUser }: AccountBindsContentProps) {
  const { user, refresh } = useCurrentUser({ initialUser });
  const [errorMessage, setErrorMessage] = useState<string | null>(null);
  const [pendingGovernorId, setPendingGovernorId] = useState<number | null>(null);

  const resolvedUser = user ?? initialUser;
  const claimedBinds = resolvedUser.claimedGovernors ?? [];
  const canClaimMore = claimedBinds.length < MAX_GOVERNOR_BINDS;

  const handleClaimed = useCallback(async () => {
    await refresh();
    setErrorMessage(null);
  }, [refresh]);

  const handleBindAction = async (governorId: number, action: BindAction) => {
    setPendingGovernorId(governorId);
    setErrorMessage(null);

    try {
      const updateBind = action === "set-default" ? setDefaultBind : unlinkBind;

      await updateBind(governorId);
      await refresh();
    } catch (error) {
      const fallback =
        action === "set-default"
          ? "Unable to set default bind. Please try again."
          : "Unable to unlink bind. Please try again.";

      setErrorMessage(error instanceof Error ? error.message : fallback);
    } finally {
      setPendingGovernorId(null);
    }
  };

  return (
    <div className="space-y-8">
      {errorMessage ? <Text className="text-red-400">{errorMessage}</Text> : null}
      <BindsList
        binds={claimedBinds}
        pendingGovernorId={pendingGovernorId}
        onSetDefault={(id) => handleBindAction(id, "set-default")}
        onUnlink={(id) => handleBindAction(id, "unlink")}
      />
      <SettingsSection
        title="Bind a governor"
        description={`Link a governor to your account by entering the Governor ID from Rise of Kingdoms. You can claim up to ${MAX_GOVERNOR_BINDS}.`}
      >
        <BindsClaimForm canClaimMore={canClaimMore} onClaimed={handleClaimed} />
      </SettingsSection>
    </div>
  );
}
