"use client";

import { useState } from "react";
import { SettingsSection } from "@/components/(app)/account/settings/settings-section";
import { Text } from "@/components/ui/text";
import { useCurrentUser } from "@/hooks/use-current-user";
import {
  type BindSuggestion,
  bindSuggestions,
  findMoreGovernors,
  revokeShare,
  setDefaultBind,
  shareBinds,
  unlinkBinds,
} from "@/lib/(app)/governor-binds";
import type { ClaimedGovernor, CurrentUser } from "@/lib/types/current-user";
import { BindActionDialog } from "./bind-action-dialog";
import { BindsClaimForm } from "./binds-claim-form";
import { BindsList } from "./binds-list";

export type BindAction =
  | { type: "share"; binds: ClaimedGovernor[] }
  | { type: "unlink"; ids: number[]; removesPrimary: boolean }
  | { type: "manage"; id: number }
  | { type: "discover"; id: number; suggestions: BindSuggestion[] };

export function AccountBindsContent({ initialUser }: { initialUser: CurrentUser }) {
  const { user, refresh } = useCurrentUser({ initialUser });
  const [selectedIds, setSelectedIds] = useState<number[]>([]);
  const [action, setAction] = useState<BindAction | null>(null);
  const [errorMessage, setErrorMessage] = useState<string | null>(null);
  const [pending, setPending] = useState(false);

  const binds = user?.claimedGovernors ?? [];
  const selection = new Set(selectedIds);
  const selected = binds.filter((bind) => selection.has(bind.governorId));
  const managed =
    action?.type === "manage" ? binds.find((bind) => bind.governorId === action.id) : undefined;

  const openAction = (next: BindAction) => {
    setErrorMessage(null);
    setAction(next);
  };

  const runUpdate = async (update: () => Promise<void>, closeOnSuccess = true) => {
    setPending(true);
    setErrorMessage(null);

    try {
      await update();

      if (closeOnSuccess) {
        setAction(null);
        setSelectedIds([]);
      }
    } catch (error) {
      setErrorMessage(error instanceof Error ? error.message : "Unable to update governor binds.");
    } finally {
      await refresh();
      setPending(false);
    }
  };

  const discover = async (id: number, afterClaim = false) => {
    setPending(true);
    setErrorMessage(null);

    try {
      const suggestions = await findMoreGovernors(id);

      if (!afterClaim || suggestions.length > 0) {
        openAction({ type: "discover", id, suggestions });
      }
    } catch (error) {
      setErrorMessage(error instanceof Error ? error.message : "Unable to find other governors.");
    } finally {
      setPending(false);
    }
  };

  const handleClaimed = async (id: number) => {
    await Promise.all([refresh(), discover(id, true)]);
  };

  const closeAction = () => {
    if (!pending) {
      setAction(null);
      setErrorMessage(null);
    }
  };

  return (
    <div className="space-y-8">
      {errorMessage && !action ? (
        <Text role="alert" className="text-red-400">
          {errorMessage}
        </Text>
      ) : null}
      <SettingsSection title="Governor Binds">
        <BindsList
          binds={binds}
          selected={selected}
          disabled={pending}
          onSelectionChange={setSelectedIds}
          onSetDefault={(id) => void runUpdate(() => setDefaultBind(id))}
          onFindMore={(id) => void discover(id)}
          onShare={(rows) => openAction({ type: "share", binds: rows })}
          onUnlink={(rows) =>
            openAction({
              type: "unlink",
              ids: rows.map((row) => row.governorId),
              removesPrimary: rows.some((row) => !row.isShared),
            })
          }
          onManageShares={(id) => openAction({ type: "manage", id })}
        />
      </SettingsSection>
      <BindsClaimForm disabled={pending} onClaimed={handleClaimed} />
      {action ? (
        <BindActionDialog
          action={action}
          managed={managed}
          pending={pending}
          errorMessage={errorMessage}
          onClose={closeAction}
          onShare={(ids, discordId) => void runUpdate(() => shareBinds(ids, discordId))}
          onUnlink={(ids) => void runUpdate(() => unlinkBinds(ids))}
          onRevoke={(id, discordId) => void runUpdate(() => revokeShare(id, discordId), false)}
          onBindSuggestions={(id, ids) => void runUpdate(() => bindSuggestions(id, ids))}
        />
      ) : null}
    </div>
  );
}
