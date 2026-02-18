"use client";

import { useExtracted } from "next-intl";
import { type FormEvent, useId, useState } from "react";
import { Button } from "@/components/ui/button";
import { Description, ErrorMessage, Field, FieldGroup, Label } from "@/components/ui/fieldset";
import { Input } from "@/components/ui/input";

type ClaimGovernorFormProps = {
  canClaimMore: boolean;
  onClaimed: () => Promise<void>;
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

export function ClaimGovernorForm({ canClaimMore, onClaimed }: ClaimGovernorFormProps) {
  const t = useExtracted();
  const tCommon = useExtracted();
  const [governorIdInput, setGovernorIdInput] = useState("");
  const [errorMessage, setErrorMessage] = useState<string | null>(null);
  const [isSubmitting, setIsSubmitting] = useState(false);
  const id = useId();

  const handleSubmit = async (event: FormEvent<HTMLFormElement>) => {
    event.preventDefault();

    if (!canClaimMore) {
      setErrorMessage(t("You can only claim up to three governors."));
      return;
    }

    const trimmed = governorIdInput.trim();
    if (trimmed === "") {
      setErrorMessage(t("Enter a governor ID."));
      return;
    }

    const numericGovernorId = Number(trimmed);
    if (!Number.isFinite(numericGovernorId)) {
      setErrorMessage(t("Enter a valid governor ID."));
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
          t("Unable to claim governor. Please try again.");

        setErrorMessage(message);
        return;
      }

      if (payload && isClaimResponse(payload) && "claim" in payload) {
        const claim = payload.claim;

        if (claim.alreadyClaimed) {
          setErrorMessage(t("This governor is already claimed."));
          return;
        }
      }

      await onClaimed();
      setGovernorIdInput("");
      setErrorMessage(null);
    } catch (error) {
      console.error("Failed to claim governor", error);
      setErrorMessage(t("Something went wrong while claiming the governor. Please try again."));
    } finally {
      setIsSubmitting(false);
    }
  };

  return (
    <form onSubmit={handleSubmit} noValidate>
      <FieldGroup>
        <Field>
          <Label htmlFor={id}>{tCommon("Governor ID")}</Label>
          <Input
            id={id}
            name="governorId"
            inputMode="numeric"
            pattern="[0-9]*"
            placeholder={tCommon("71738515")}
            value={governorIdInput}
            onChange={(event) => {
              setGovernorIdInput(event.target.value);
              if (errorMessage) {
                setErrorMessage(null);
              }
            }}
            disabled={isSubmitting || !canClaimMore}
            autoComplete="off"
          />
          {errorMessage ? (
            <ErrorMessage>{errorMessage}</ErrorMessage>
          ) : !canClaimMore ? (
            <Description>
              {t("You have reached the maximum of three claimed governors.")}
            </Description>
          ) : undefined}
        </Field>
      </FieldGroup>
      <div className="mt-4 flex items-center gap-3">
        <Button
          type="submit"
          disabled={!canClaimMore || isSubmitting || governorIdInput.trim() === ""}
        >
          {isSubmitting ? t("Claiming...") : t("Claim governor")}
        </Button>
      </div>
    </form>
  );
}
