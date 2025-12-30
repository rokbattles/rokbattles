"use client";

import { useTranslations } from "next-intl";
import { type FormEvent, useId, useState } from "react";
import { Button } from "@/components/ui/Button";
import { Description, ErrorMessage, Field, FieldGroup, Label } from "@/components/ui/Fieldset";
import { Input } from "@/components/ui/Input";

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
  const t = useTranslations("account");
  const tCommon = useTranslations("common");
  const [governorIdInput, setGovernorIdInput] = useState("");
  const [errorMessage, setErrorMessage] = useState<string | null>(null);
  const [isSubmitting, setIsSubmitting] = useState(false);
  const id = useId();

  const handleSubmit = async (event: FormEvent<HTMLFormElement>) => {
    event.preventDefault();

    if (!canClaimMore) {
      setErrorMessage(t("claimForm.errors.maxReached"));
      return;
    }

    const trimmed = governorIdInput.trim();
    if (trimmed === "") {
      setErrorMessage(t("claimForm.errors.missingId"));
      return;
    }

    const numericGovernorId = Number(trimmed);
    if (!Number.isFinite(numericGovernorId)) {
      setErrorMessage(t("claimForm.errors.invalidId"));
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
          t("claimForm.errors.unavailable");

        setErrorMessage(message);
        return;
      }

      if (payload && isClaimResponse(payload) && "claim" in payload) {
        const claim = payload.claim;

        if (claim.alreadyClaimed) {
          setErrorMessage(t("claimForm.errors.alreadyClaimed"));
          return;
        }
      }

      await onClaimed();
      setGovernorIdInput("");
      setErrorMessage(null);
    } catch (error) {
      console.error("Failed to claim governor", error);
      setErrorMessage(t("claimForm.errors.unknown"));
    } finally {
      setIsSubmitting(false);
    }
  };

  return (
    <form onSubmit={handleSubmit} noValidate>
      <FieldGroup>
        <Field>
          <Label htmlFor={id}>{tCommon("fields.governorId")}</Label>
          <Input
            id={id}
            name="governorId"
            inputMode="numeric"
            pattern="[0-9]*"
            placeholder={tCommon("placeholders.governorId")}
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
            <Description>{t("claimForm.maxReached")}</Description>
          ) : undefined}
        </Field>
      </FieldGroup>
      <div className="mt-4 flex items-center gap-3">
        <Button
          type="submit"
          disabled={!canClaimMore || isSubmitting || governorIdInput.trim() === ""}
        >
          {isSubmitting ? t("claimForm.submitting") : t("claimForm.submit")}
        </Button>
      </div>
    </form>
  );
}
