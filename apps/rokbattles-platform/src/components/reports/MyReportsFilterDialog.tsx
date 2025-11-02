"use client";

import type React from "react";
import { useContext, useEffect, useState } from "react";
import {
  ReportsFilterContext,
  type ReportsFilterType,
} from "@/components/context/ReportsFilterContext";
import { Button } from "@/components/ui/Button";
import {
  Dialog,
  DialogActions,
  DialogBody,
  DialogDescription,
  DialogTitle,
} from "@/components/ui/Dialog";
import { Field, FieldGroup, Label } from "@/components/ui/Fieldset";
import { Select } from "@/components/ui/Select";
import { useCommanderOptions } from "@/hooks/useCommanderName";

export function MyReportsFilterDialog(props: React.ComponentPropsWithoutRef<typeof Button>) {
  const context = useContext(ReportsFilterContext);
  if (!context) {
    throw new Error("MyReportsFilterDialog must be used within a ReportsFilterProvider");
  }

  const {
    type,
    setType,
    primaryCommanderId,
    setPrimaryCommanderId,
    secondaryCommanderId,
    setSecondaryCommanderId,
  } = context;

  const [isOpen, setIsOpen] = useState(false);
  const [localType, setLocalType] = useState<ReportsFilterType | "">(() => type ?? "");
  const [localPrimaryCommanderId, setLocalPrimaryCommanderId] = useState(() =>
    typeof primaryCommanderId === "number" ? String(primaryCommanderId) : ""
  );
  const [localSecondaryCommanderId, setLocalSecondaryCommanderId] = useState(() =>
    typeof secondaryCommanderId === "number" ? String(secondaryCommanderId) : ""
  );

  const commanderOptions = useCommanderOptions();

  useEffect(() => {
    if (!isOpen) {
      return;
    }

    setLocalType(type ?? "");
    setLocalPrimaryCommanderId(
      typeof primaryCommanderId === "number" ? String(primaryCommanderId) : ""
    );
    setLocalSecondaryCommanderId(
      typeof secondaryCommanderId === "number" ? String(secondaryCommanderId) : ""
    );
  }, [isOpen, type, primaryCommanderId, secondaryCommanderId]);

  const handleApply = () => {
    const trimmedPrimaryCommanderId = localPrimaryCommanderId.trim();
    const primaryCommanderNumericId = Number(trimmedPrimaryCommanderId);
    const nextPrimaryCommanderId =
      trimmedPrimaryCommanderId === "" || !Number.isFinite(primaryCommanderNumericId)
        ? undefined
        : Math.trunc(primaryCommanderNumericId);

    const trimmedSecondaryCommanderId = localSecondaryCommanderId.trim();
    const secondaryCommanderNumericId = Number(trimmedSecondaryCommanderId);
    const nextSecondaryCommanderId =
      trimmedSecondaryCommanderId === "" || !Number.isFinite(secondaryCommanderNumericId)
        ? undefined
        : Math.trunc(secondaryCommanderNumericId);

    setType(localType === "" ? undefined : localType);
    setPrimaryCommanderId(nextPrimaryCommanderId);
    setSecondaryCommanderId(nextSecondaryCommanderId);
    setIsOpen(false);
  };

  return (
    <>
      <Button type="button" onClick={() => setIsOpen(true)} {...props} />
      <Dialog open={isOpen} onClose={setIsOpen}>
        <DialogTitle>Filters</DialogTitle>
        <DialogDescription>Filter your claimed governor reports by type.</DialogDescription>
        <DialogBody>
          <FieldGroup>
            <Field>
              <Label>Type</Label>
              <Select
                value={localType}
                onChange={(event) => {
                  const nextValue = event.target.value;
                  setLocalType(nextValue === "" ? "" : (nextValue as ReportsFilterType));
                }}
              >
                <option value="">All</option>
                <option value="kvk">KVK</option>
                <option value="ark">Ark of Osiris</option>
              </Select>
            </Field>
            <Field>
              <Label>Primary Commander</Label>
              <Select
                value={localPrimaryCommanderId}
                onChange={(event) => {
                  setLocalPrimaryCommanderId(event.target.value);
                }}
              >
                <option value="">All</option>
                {commanderOptions.map((option) => (
                  <option key={option.id} value={String(option.id)}>
                    {option.name}
                  </option>
                ))}
              </Select>
            </Field>
            <Field>
              <Label>Secondary Commander</Label>
              <Select
                value={localSecondaryCommanderId}
                onChange={(event) => {
                  setLocalSecondaryCommanderId(event.target.value);
                }}
              >
                <option value="">All</option>
                {commanderOptions.map((option) => (
                  <option key={option.id} value={String(option.id)}>
                    {option.name}
                  </option>
                ))}
              </Select>
            </Field>
          </FieldGroup>
        </DialogBody>
        <DialogActions>
          <Button plain onClick={() => setIsOpen(false)}>
            Cancel
          </Button>
          <Button onClick={handleApply}>Apply</Button>
        </DialogActions>
      </Dialog>
    </>
  );
}
