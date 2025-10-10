"use client";

import { ChevronDownIcon, PlusIcon } from "@heroicons/react/16/solid";
import { type FormEvent, useContext, useId, useState } from "react";
import { GovernorContext } from "@/components/context/GovernorContext";
import { Avatar } from "@/components/ui/Avatar";
import { Button } from "@/components/ui/Button";
import {
  Dialog,
  DialogActions,
  DialogBody,
  DialogDescription,
  DialogTitle,
} from "@/components/ui/Dialog";
import {
  Dropdown,
  DropdownButton,
  DropdownDescription,
  DropdownDivider,
  DropdownItem,
  DropdownLabel,
  DropdownMenu,
} from "@/components/ui/Dropdown";
import { ErrorMessage, Field, FieldGroup, Label } from "@/components/ui/Fieldset";
import { Input } from "@/components/ui/Input";
import { SidebarHeader, SidebarItem, SidebarLabel } from "@/components/ui/Sidebar";
import type { CurrentUser } from "@/hooks/useCurrentUser";

type SidebarGovernorHeaderProps = {
  user: CurrentUser;
  onRefresh: () => Promise<void>;
};

type ClaimResponse =
  | {
      claim: {
        governorId: number;
        governorName: string | null;
        governorAvatar: string | null;
        alreadyClaimed?: boolean;
      };
    }
  | { error?: string };

function isClaimResponse(payload: unknown): payload is ClaimResponse {
  if (!payload || typeof payload !== "object") {
    return false;
  }

  if ("claim" in payload && payload.claim && typeof payload.claim === "object") {
    const claim = payload.claim as Record<string, unknown>;
    return typeof claim.governorId === "number";
  }

  if ("error" in payload && typeof (payload as { error?: unknown }).error === "string") {
    return true;
  }

  return false;
}

export function SidebarGovernorHeader({ user, onRefresh }: SidebarGovernorHeaderProps) {
  const [isOpen, setIsOpen] = useState(false);
  const [governorIdInput, setGovernorIdInput] = useState("");
  const [errorMessage, setErrorMessage] = useState<string | null>(null);
  const [isSubmitting, setIsSubmitting] = useState(false);
  const id = useId();

  const context = useContext(GovernorContext);
  if (!context) {
    throw new Error("SidebarGovernorHeader must be used within a GovernorProvider");
  }

  const { activeGovernor, governors, selectGovernor } = context;

  const displayName = activeGovernor
    ? (activeGovernor.governorName ?? activeGovernor.governorId.toString())
    : "Select a governor";
  const displayAvatar = activeGovernor?.governorAvatar ?? null;
  const canClaimMore = user.claimedGovernors.length < 3;

  const resetFormState = () => {
    setGovernorIdInput("");
    setErrorMessage(null);
  };

  const handleDialogToggle = (nextOpen: boolean) => {
    if (!nextOpen) {
      resetFormState();
    }

    setIsOpen(nextOpen);
  };

  const handleSubmit = async (event: FormEvent<HTMLFormElement>) => {
    event.preventDefault();

    if (!canClaimMore) {
      setErrorMessage("You can only claim up to three governors.");
      return;
    }

    const trimmed = governorIdInput.trim();
    if (trimmed === "") {
      setErrorMessage("Enter a governor ID.");
      return;
    }

    const numericGovernorId = Number.parseInt(trimmed, 10);
    if (!Number.isFinite(numericGovernorId) || numericGovernorId <= 0) {
      setErrorMessage("Enter a valid governor ID.");
      return;
    }

    setIsSubmitting(true);
    setErrorMessage(null);

    try {
      const response = await fetch("/api/v2/governor/claim", {
        method: "POST",
        headers: {
          "Content-Type": "application/json",
        },
        body: JSON.stringify({ governorId: numericGovernorId }),
      });

      let payload: unknown = null;
      try {
        payload = await response.json();
      } catch {
        payload = null;
      }

      if (!response.ok) {
        const message =
          (payload &&
            typeof payload === "object" &&
            "error" in payload &&
            typeof (payload as { error?: unknown }).error === "string" &&
            (payload as { error?: string }).error) ||
          "Unable to claim governor. Please try again.";

        setErrorMessage(message);
        return;
      }

      if (payload && isClaimResponse(payload) && "claim" in payload) {
        const claim = payload.claim;

        if (claim.alreadyClaimed) {
          setErrorMessage("This governor is already claimed.");
          return;
        }
      }

      await onRefresh();
      setIsOpen(false);
      resetFormState();
    } catch (error) {
      console.error("Failed to claim governor", error);
      setErrorMessage("Something went wrong while claiming the governor. Please try again.");
    } finally {
      setIsSubmitting(false);
    }
  };

  return (
    <>
      <SidebarHeader>
        <Dropdown>
          <DropdownButton as={SidebarItem}>
            {displayAvatar && <Avatar slot="icon" src={displayAvatar} className="size-10" square />}
            <SidebarLabel>{displayName}</SidebarLabel>
            <ChevronDownIcon />
          </DropdownButton>
          <DropdownMenu className="min-w-80 lg:min-w-64" anchor="bottom start">
            {governors.length > 0 &&
              governors.map((claim) => (
                <DropdownItem
                  key={claim.governorId}
                  onClick={() => selectGovernor(claim.governorId)}
                >
                  {claim.governorAvatar && <Avatar slot="icon" src={claim.governorAvatar} square />}
                  <DropdownLabel>{claim.governorName ?? claim.governorId.toString()}</DropdownLabel>
                  {claim.governorName && (
                    <DropdownDescription>{claim.governorId}</DropdownDescription>
                  )}
                </DropdownItem>
              ))}
            {governors.length < 3 && canClaimMore && (
              <DropdownItem
                onClick={() => {
                  resetFormState();
                  setIsOpen(true);
                }}
              >
                <PlusIcon />
                <DropdownLabel>Claim governor&hellip;</DropdownLabel>
              </DropdownItem>
            )}
            <DropdownDivider />
            <DropdownItem disabled>
              <PlusIcon />
              <DropdownLabel>New group&hellip;</DropdownLabel>
            </DropdownItem>
          </DropdownMenu>
        </Dropdown>
      </SidebarHeader>
      <Dialog open={isOpen} onClose={handleDialogToggle}>
        <DialogTitle>Claim a governor</DialogTitle>
        <DialogDescription>
          Link a governor to your account by entering the ID. You can claim up to three governors.
        </DialogDescription>
        <form onSubmit={handleSubmit} noValidate>
          <DialogBody>
            <FieldGroup>
              <Field>
                <Label htmlFor={id}>Governor ID</Label>
                <Input
                  id={id}
                  name="governorId"
                  inputMode="numeric"
                  pattern="[0-9]*"
                  placeholder="71738515"
                  value={governorIdInput}
                  onChange={(event) => {
                    setGovernorIdInput(event.target.value);
                    if (errorMessage) {
                      setErrorMessage(null);
                    }
                  }}
                  disabled={isSubmitting}
                  autoComplete="off"
                  autoFocus
                />
                {errorMessage ? <ErrorMessage>{errorMessage}</ErrorMessage> : null}
              </Field>
            </FieldGroup>
          </DialogBody>
          <DialogActions>
            <Button
              plain
              type="button"
              onClick={() => handleDialogToggle(false)}
              disabled={isSubmitting}
            >
              Cancel
            </Button>
            <Button type="submit" disabled={isSubmitting || governorIdInput.trim() === ""}>
              {isSubmitting ? "Claiming..." : "Claim governor"}
            </Button>
          </DialogActions>
        </form>
      </Dialog>
    </>
  );
}
