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
import { Input } from "@/components/ui/Input";
import { Listbox, ListboxLabel, ListboxOption } from "@/components/ui/Listbox";
import { useCommanderOptions } from "@/hooks/useCommanderName";

function parse(s: string) {
  const t = s.trim();
  if (t === "") return undefined;
  const n = Number(t);
  return Number.isFinite(n) ? Math.trunc(n) : undefined;
}

export function ReportsFilterDialog(props: React.ComponentPropsWithoutRef<typeof Button>) {
  const context = useContext(ReportsFilterContext);
  if (!context) throw new Error("ReportsFilterDialog must be used within a ReportsFilterProvider");

  const {
    playerId,
    setPlayerId,
    type,
    setType,
    primaryCommanderId,
    setPrimaryCommanderId,
    secondaryCommanderId,
    setSecondaryCommanderId,
    reset,
  } = context;

  const [isOpen, setIsOpen] = useState(false);
  const [localPlayerId, setLocalPlayerId] = useState(() =>
    typeof playerId === "number" ? String(playerId) : ""
  );
  const [localType, setLocalType] = useState<ReportsFilterType | "">(() => type ?? "");
  const [localPrimaryCommanderId, setLocalPrimaryCommanderId] = useState(() =>
    typeof primaryCommanderId === "number" ? String(primaryCommanderId) : ""
  );
  const [localSecondaryCommanderId, setLocalSecondaryCommanderId] = useState(() =>
    typeof secondaryCommanderId === "number" ? String(secondaryCommanderId) : ""
  );

  const commanderOptions = useCommanderOptions();

  useEffect(() => {
    if (!isOpen) return;
    setLocalPlayerId(typeof playerId === "number" ? String(playerId) : "");
    setLocalType(type ?? "");
    setLocalPrimaryCommanderId(
      typeof primaryCommanderId === "number" ? String(primaryCommanderId) : ""
    );
    setLocalSecondaryCommanderId(
      typeof secondaryCommanderId === "number" ? String(secondaryCommanderId) : ""
    );
  }, [isOpen, playerId, type, primaryCommanderId, secondaryCommanderId]);

  const handleApply = () => {
    setPlayerId(parse(localPlayerId));
    setType(localType === "" ? undefined : localType);
    setPrimaryCommanderId(parse(localPrimaryCommanderId));
    setSecondaryCommanderId(parse(localSecondaryCommanderId));
    setIsOpen(false);
  };

  return (
    <>
      <Button type="button" onClick={() => setIsOpen(true)} {...props} />
      <Dialog open={isOpen} onClose={setIsOpen}>
        <DialogTitle>Filters</DialogTitle>
        <DialogDescription>Filter battle reports list by type and/or governor.</DialogDescription>
        <DialogBody>
          <FieldGroup>
            <Field>
              <Label>Governor ID</Label>
              <Input
                inputMode="numeric"
                pattern="[0-9]*"
                placeholder="71738515"
                value={localPlayerId}
                onChange={(event) => {
                  setLocalPlayerId(event.target.value);
                }}
              />
            </Field>
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
