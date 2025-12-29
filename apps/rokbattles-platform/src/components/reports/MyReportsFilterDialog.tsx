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
import { Listbox, ListboxLabel, ListboxOption } from "@/components/ui/Listbox";
import { Switch, SwitchField } from "@/components/ui/Switch";
import { useCommanderOptions } from "@/hooks/useCommanderName";

function parseNumberInput(value: string) {
  const trimmed = value.trim();
  if (trimmed === "") {
    return undefined;
  }
  const numeric = Number(trimmed);
  return Number.isFinite(numeric) ? numeric : undefined;
}

export function MyReportsFilterDialog(props: React.ComponentPropsWithoutRef<typeof Button>) {
  const context = useContext(ReportsFilterContext);
  if (!context)
    throw new Error("MyReportsFilterDialog must be used within a ReportsFilterProvider");

  const {
    type,
    setType,
    rallyOnly,
    setRallyOnly,
    primaryCommanderId,
    setPrimaryCommanderId,
    secondaryCommanderId,
    setSecondaryCommanderId,
    reset,
  } = context;

  const [isOpen, setIsOpen] = useState(false);
  const [localType, setLocalType] = useState<ReportsFilterType | "">(() => type ?? "");
  const [localRallyOnly, setLocalRallyOnly] = useState(() => rallyOnly);
  const [localPrimaryCommanderId, setLocalPrimaryCommanderId] = useState(() =>
    typeof primaryCommanderId === "number" ? String(primaryCommanderId) : ""
  );
  const [localSecondaryCommanderId, setLocalSecondaryCommanderId] = useState(() =>
    typeof secondaryCommanderId === "number" ? String(secondaryCommanderId) : ""
  );

  const commanderOptions = useCommanderOptions();

  useEffect(() => {
    if (!isOpen) return;
    setLocalType(type ?? "");
    setLocalRallyOnly(rallyOnly);
    setLocalPrimaryCommanderId(
      typeof primaryCommanderId === "number" ? String(primaryCommanderId) : ""
    );
    setLocalSecondaryCommanderId(
      typeof secondaryCommanderId === "number" ? String(secondaryCommanderId) : ""
    );
  }, [isOpen, type, rallyOnly, primaryCommanderId, secondaryCommanderId]);

  const handleApply = () => {
    setType(localType === "" ? undefined : localType);
    setRallyOnly(localRallyOnly);
    setPrimaryCommanderId(parseNumberInput(localPrimaryCommanderId));
    setSecondaryCommanderId(parseNumberInput(localSecondaryCommanderId));
    setIsOpen(false);
  };

  return (
    <>
      <Button type="button" onClick={() => setIsOpen(true)} {...props} />
      <Dialog open={isOpen} onClose={setIsOpen}>
        <DialogTitle>Filters</DialogTitle>
        <DialogDescription>Filter your battle reports by type.</DialogDescription>
        <DialogBody>
          <FieldGroup>
            <Field>
              <Label>Type</Label>
              <Listbox<ReportsFilterType | "">
                value={localType}
                onChange={(value) => {
                  setLocalType(value);
                }}
              >
                <ListboxOption value="">
                  <ListboxLabel>All</ListboxLabel>
                </ListboxOption>
                <ListboxOption value="kvk">
                  <ListboxLabel>KVK</ListboxLabel>
                </ListboxOption>
                <ListboxOption value="ark">
                  <ListboxLabel>Ark of Osiris</ListboxLabel>
                </ListboxOption>
              </Listbox>
            </Field>
            <Field>
              <Label>Primary Commander</Label>
              <Listbox
                value={localPrimaryCommanderId}
                onChange={(value) => {
                  setLocalPrimaryCommanderId(value);
                }}
              >
                <ListboxOption value="">
                  <ListboxLabel>All</ListboxLabel>
                </ListboxOption>
                {commanderOptions.map((option) => (
                  <ListboxOption key={option.id} value={String(option.id)}>
                    <ListboxLabel>{option.name}</ListboxLabel>
                  </ListboxOption>
                ))}
              </Listbox>
            </Field>
            <Field>
              <Label>Secondary Commander</Label>
              <Listbox
                value={localSecondaryCommanderId}
                onChange={(value) => {
                  setLocalSecondaryCommanderId(value);
                }}
              >
                <ListboxOption value="">
                  <ListboxLabel>All</ListboxLabel>
                </ListboxOption>
                {commanderOptions.map((option) => (
                  <ListboxOption key={option.id} value={String(option.id)}>
                    <ListboxLabel>{option.name}</ListboxLabel>
                  </ListboxOption>
                ))}
              </Listbox>
            </Field>
            <SwitchField>
              <Label>Garrison/Rally reports only</Label>
              <Switch
                checked={localRallyOnly}
                onChange={(checked) => {
                  setLocalRallyOnly(checked);
                }}
              />
            </SwitchField>
          </FieldGroup>
        </DialogBody>
        <DialogActions>
          <Button plain onClick={() => setIsOpen(false)}>
            Cancel
          </Button>
          <Button
            plain
            onClick={() => {
              reset();
              setIsOpen(false);
            }}
          >
            Reset
          </Button>
          <Button onClick={handleApply}>Apply</Button>
        </DialogActions>
      </Dialog>
    </>
  );
}
