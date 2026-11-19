"use client";

import { type FormEvent, useId, useState } from "react";
import { Avatar } from "@/components/ui/avatar";
import { Button } from "@/components/ui/button";
import { Checkbox, CheckboxField, CheckboxGroup } from "@/components/ui/checkbox";
import {
  Dialog,
  DialogActions,
  DialogBody,
  DialogDescription,
  DialogTitle,
} from "@/components/ui/dialog";
import { Field, Label } from "@/components/ui/fieldset";
import { Input } from "@/components/ui/input";
import { Text } from "@/components/ui/text";
import type { ClaimedGovernor } from "@/lib/types/current-user";
import type { BindAction } from "./account-binds-content";

type BindActionDialogProps = {
  action: BindAction;
  managed?: ClaimedGovernor;
  pending: boolean;
  errorMessage: string | null;
  onClose: () => void;
  onShare: (ids: number[], discordId: string) => void;
  onUnlink: (ids: number[]) => void;
  onRevoke: (id: number, discordId: string) => void;
  onBindSuggestions: (id: number, ids: number[]) => void;
};

export function BindActionDialog({
  action,
  managed,
  pending,
  errorMessage,
  onClose,
  onShare,
  onUnlink,
  onRevoke,
  onBindSuggestions,
}: BindActionDialogProps) {
  const discordInputId = useId();
  const [discordId, setDiscordId] = useState("");
  const [selected, setSelected] = useState<number[]>([]);
  const [revoking, setRevoking] = useState<string | null>(null);

  const handleSubmit = (event: FormEvent<HTMLFormElement>) => {
    event.preventDefault();
    if (pending) {
      return;
    }

    switch (action.type) {
      case "share":
        onShare(
          action.binds.map((bind) => bind.governorId),
          discordId.trim()
        );
        break;
      case "unlink":
        onUnlink(action.ids);
        break;
      case "discover":
        onBindSuggestions(action.id, selected);
        break;
      case "manage":
        if (revoking) {
          onRevoke(action.id, revoking);
          setRevoking(null);
        }
        break;
    }
  };

  const hasSuggestions = action.type === "discover" && action.suggestions.length > 0;
  const showDone =
    (action.type === "manage" && !revoking) || (action.type === "discover" && !hasSuggestions);
  const closeLabel = showDone ? "Done" : hasSuggestions ? "Skip" : "Cancel";

  return (
    <Dialog open onClose={onClose}>
      <form onSubmit={handleSubmit}>
        {action.type === "share" ? (
          <>
            <DialogTitle>Share</DialogTitle>
            <DialogDescription>
              The recipient will be able to use these governors.
              <span className="mt-1 block wrap-anywhere font-medium text-zinc-200">
                {action.binds
                  .map((bind) => bind.governorName ?? `Governor ${bind.governorId}`)
                  .join(", ")}
              </span>
            </DialogDescription>
            <DialogBody>
              <Field>
                <Label htmlFor={discordInputId}>Discord ID</Label>
                <Input
                  id={discordInputId}
                  name="discordId"
                  type="text"
                  inputMode="numeric"
                  pattern="[1-9][0-9]*"
                  maxLength={20}
                  required
                  disabled={pending}
                  value={discordId}
                  onChange={(event) => setDiscordId(event.currentTarget.value.trim())}
                />
              </Field>
            </DialogBody>
          </>
        ) : null}
        {action.type === "unlink" ? (
          <>
            <DialogTitle>Unbind {governorCount(action.ids.length)}</DialogTitle>
            <DialogDescription>
              {action.removesPrimary
                ? "Unbinding a primary governor also removes access for everyone it was shared with."
                : "You will lose access to the selected shared governors."}
            </DialogDescription>
          </>
        ) : null}
        {action.type === "discover" ? (
          <>
            <DialogTitle>Bind related governors</DialogTitle>
            <DialogDescription>
              {hasSuggestions
                ? "We found these related governors. Would you like to bind any of them?"
                : "There are no related governors available to bind."}
            </DialogDescription>
            {hasSuggestions ? (
              <DialogBody className="max-h-72 overflow-y-auto">
                <CheckboxGroup>
                  {action.suggestions.map((suggestion) => {
                    const id = suggestion.governorId;
                    const name = suggestion.governorName ?? `Governor ${id}`;
                    return (
                      <CheckboxField key={id}>
                        <Checkbox
                          checked={selected.includes(id)}
                          disabled={pending}
                          onChange={(checked) =>
                            setSelected((current) =>
                              checked ? [...current, id] : current.filter((value) => value !== id)
                            )
                          }
                        />
                        <Label className="flex items-center gap-2">
                          <Avatar
                            src={suggestion.governorAvatar}
                            initials={name[0]}
                            className="size-7"
                            square
                          />
                          <span>{name}</span>
                          {suggestion.governorName ? (
                            <span className="text-xs text-zinc-400">{id}</span>
                          ) : null}
                        </Label>
                      </CheckboxField>
                    );
                  })}
                </CheckboxGroup>
              </DialogBody>
            ) : null}
          </>
        ) : null}
        {action.type === "manage" ? (
          <>
            <DialogTitle>Manage access</DialogTitle>
            <DialogDescription>
              {revoking
                ? `Revoke access for Discord ID ${revoking}? Their authorized apps will lose access to this governor too.`
                : `Shared access for ${managed?.governorName ?? `Governor ${action.id}`}.`}
            </DialogDescription>
            {!revoking ? (
              <DialogBody>
                {managed?.sharedWith?.length ? (
                  <ul className="divide-y divide-white/10">
                    {managed.sharedWith.map((recipient) => (
                      <li
                        key={recipient}
                        className="flex flex-wrap items-center justify-between gap-2 py-2"
                      >
                        <Text className="tabular-nums">{recipient}</Text>
                        <Button
                          type="button"
                          outline
                          disabled={pending}
                          aria-label={`Revoke access for ${recipient}`}
                          onClick={() => setRevoking(recipient)}
                        >
                          Revoke access
                        </Button>
                      </li>
                    ))}
                  </ul>
                ) : (
                  <Text>This governor has no active shares.</Text>
                )}
              </DialogBody>
            ) : null}
          </>
        ) : null}
        {errorMessage ? (
          <Text className="mt-4 text-red-400" role="alert">
            {errorMessage}
          </Text>
        ) : null}
        <DialogActions>
          <Button
            type="button"
            plain
            disabled={pending}
            onClick={() => (revoking ? setRevoking(null) : onClose())}
          >
            {closeLabel}
          </Button>
          {action.type === "share" ? (
            <Button type="submit" disabled={pending || !discordId.trim()}>
              {pending ? "Sharing…" : "Share"}
            </Button>
          ) : null}
          {action.type === "unlink" ? (
            <Button type="submit" color="red" disabled={pending}>
              {pending ? "Unbinding…" : "Unbind"}
            </Button>
          ) : null}
          {hasSuggestions ? (
            <Button type="submit" disabled={pending || selected.length === 0}>
              {pending ? "Binding…" : "Bind selected"}
            </Button>
          ) : null}
          {action.type === "manage" && revoking ? (
            <Button type="submit" color="red" disabled={pending}>
              Revoke access
            </Button>
          ) : null}
        </DialogActions>
      </form>
    </Dialog>
  );
}

function governorCount(count: number) {
  return `${count} ${count === 1 ? "governor" : "governors"}`;
}
