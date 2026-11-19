"use client";

import { type FormEvent, useId, useState } from "react";
import { Button } from "@/components/ui/button";
import { ErrorMessage, Field, FieldGroup, Label } from "@/components/ui/fieldset";
import { Input } from "@/components/ui/input";
import { claimBind } from "@/lib/(app)/governor-binds";

type ClaimBindFormProps = {
  disabled: boolean;
  onClaimed: (governorId: number) => Promise<void>;
};

export function BindsClaimForm({ disabled, onClaimed }: ClaimBindFormProps) {
  const [governorIdInput, setGovernorIdInput] = useState("");
  const [errorMessage, setErrorMessage] = useState<string | null>(null);
  const [isSubmitting, setIsSubmitting] = useState(false);
  const id = useId();

  const handleSubmit = async (event: FormEvent<HTMLFormElement>) => {
    event.preventDefault();

    const trimmed = governorIdInput.trim();
    const governorId = Number(trimmed);

    if (!/^\d+$/.test(trimmed) || !Number.isSafeInteger(governorId) || governorId <= 0) {
      setErrorMessage("Enter a valid governor ID.");
      return;
    }

    setIsSubmitting(true);
    setErrorMessage(null);

    try {
      await claimBind(governorId);
      setGovernorIdInput("");
      await onClaimed(governorId);
    } catch (error) {
      setErrorMessage(error instanceof Error ? error.message : "Unable to bind this governor.");
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
              setErrorMessage(null);
            }}
            disabled={isSubmitting || disabled}
            autoComplete="off"
          />
          {errorMessage ? <ErrorMessage>{errorMessage}</ErrorMessage> : null}
        </Field>
      </FieldGroup>
      <Button
        className="mt-4"
        type="submit"
        disabled={disabled || isSubmitting || governorIdInput.trim() === ""}
      >
        {isSubmitting ? "Binding…" : "Bind governor"}
      </Button>
    </form>
  );
}
