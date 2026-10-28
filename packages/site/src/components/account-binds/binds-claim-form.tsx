"use client";

import { type FormEvent, useId, useState } from "react";
import { Button } from "@/components/ui/button";
import { Description, ErrorMessage, Field, FieldGroup, Label } from "@/components/ui/fieldset";
import { Input } from "@/components/ui/input";
import { claimBind } from "@/lib/account-binds/api";
import { MAX_GOVERNOR_BINDS } from "@/lib/account-binds/constants";

type ClaimBindFormProps = {
  canClaimMore: boolean;
  onClaimed: () => Promise<void>;
};

export function BindsClaimForm({ canClaimMore, onClaimed }: ClaimBindFormProps) {
  const [governorIdInput, setGovernorIdInput] = useState("");
  const [errorMessage, setErrorMessage] = useState<string | null>(null);
  const [isSubmitting, setIsSubmitting] = useState(false);
  const id = useId();

  const handleSubmit = async (event: FormEvent<HTMLFormElement>) => {
    event.preventDefault();

    if (!canClaimMore) {
      setErrorMessage(`You can only bind up to ${MAX_GOVERNOR_BINDS.toString()} governors.`);
      return;
    }

    const trimmed = governorIdInput.trim();
    if (trimmed === "") {
      setErrorMessage("Enter a governor ID.");
      return;
    }

    const numericGovernorId = Number(trimmed);
    if (!Number.isFinite(numericGovernorId)) {
      setErrorMessage("Enter a valid governor ID.");
      return;
    }

    setIsSubmitting(true);
    setErrorMessage(null);

    try {
      await claimBind(numericGovernorId);
      await onClaimed();
      setGovernorIdInput("");
      setErrorMessage(null);
    } catch (error) {
      const message =
        error instanceof Error
          ? error.message
          : "Something went wrong while binding. Please try again.";
      setErrorMessage(message);
    } finally {
      setIsSubmitting(false);
    }
  };

  return (
    <form onSubmit={handleSubmit} noValidate>
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
            disabled={isSubmitting || !canClaimMore}
            autoComplete="off"
          />
          {errorMessage ? (
            <ErrorMessage>{errorMessage}</ErrorMessage>
          ) : !canClaimMore ? (
            <Description>
              {`You have reached the maximum of ${MAX_GOVERNOR_BINDS.toString()} binds.`}
            </Description>
          ) : undefined}
        </Field>
      </FieldGroup>
      <div className="mt-4 flex items-center gap-3">
        <Button
          type="submit"
          disabled={!canClaimMore || isSubmitting || governorIdInput.trim() === ""}
        >
          {isSubmitting ? "Binding..." : "Bind governor"}
        </Button>
      </div>
    </form>
  );
}
