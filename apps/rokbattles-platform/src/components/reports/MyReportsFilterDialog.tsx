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

export function MyReportsFilterDialog(props: React.ComponentPropsWithoutRef<typeof Button>) {
  const context = useContext(ReportsFilterContext);
  if (!context) {
    throw new Error("MyReportsFilterDialog must be used within a ReportsFilterProvider");
  }

  const { type, setType } = context;

  const [isOpen, setIsOpen] = useState(false);
  const [localType, setLocalType] = useState<ReportsFilterType | "">(() => type ?? "");

  useEffect(() => {
    if (!isOpen) {
      return;
    }

    setLocalType(type ?? "");
  }, [isOpen, type]);

  const handleApply = () => {
    setType(localType === "" ? undefined : localType);
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
