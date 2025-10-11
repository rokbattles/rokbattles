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
import { Select } from "@/components/ui/Select";
import { useCommanderOptions } from "@/hooks/useCommanderName";

export function ReportsFilterDialog(props: React.ComponentPropsWithoutRef<typeof Button>) {
  const context = useContext(ReportsFilterContext);
  if (!context) {
    throw new Error("ReportsFilterDialog must be used within a ReportsFilterProvider");
  }

  const { playerId, setPlayerId, type, setType, commanderId, setCommanderId } = context;

  const [isOpen, setIsOpen] = useState(false);
  const [localPlayerId, setLocalPlayerId] = useState(() =>
    typeof playerId === "number" ? String(playerId) : ""
  );
  const [localType, setLocalType] = useState<ReportsFilterType | "">(() => type ?? "");
  const [localCommanderId, setLocalCommanderId] = useState(() =>
    typeof commanderId === "number" ? String(commanderId) : ""
  );

  const commanderOptions = useCommanderOptions();

  useEffect(() => {
    if (!isOpen) {
      return;
    }

    setLocalPlayerId(typeof playerId === "number" ? String(playerId) : "");
    setLocalType(type ?? "");
    setLocalCommanderId(typeof commanderId === "number" ? String(commanderId) : "");
  }, [isOpen, playerId, type, commanderId]);

  const handleApply = () => {
    const trimmedId = localPlayerId.trim();
    const numericId = Number(trimmedId);
    const nextPlayerId =
      trimmedId === "" || !Number.isFinite(numericId) ? undefined : Math.trunc(numericId);

    const trimmedCommanderId = localCommanderId.trim();
    const commanderNumericId = Number(trimmedCommanderId);
    const nextCommanderId =
      trimmedCommanderId === "" || !Number.isFinite(commanderNumericId)
        ? undefined
        : Math.trunc(commanderNumericId);

    setPlayerId(nextPlayerId);
    setType(localType === "" ? undefined : localType);
    setCommanderId(nextCommanderId);
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
              <Label>Commander</Label>
              <Select
                value={localCommanderId}
                onChange={(event) => {
                  setLocalCommanderId(event.target.value);
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
