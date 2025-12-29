"use client";

import type React from "react";
import { useContext, useEffect, useState } from "react";
import {
  ReportsFilterContext,
  type ReportsFilterSide,
  type ReportsFilterType,
  type ReportsGarrisonBuildingType,
} from "@/components/context/ReportsFilterContext";
import { Button } from "@/components/ui/Button";
import {
  Dialog,
  DialogActions,
  DialogBody,
  DialogDescription,
  DialogTitle,
} from "@/components/ui/Dialog";
import { Field, Fieldset, Label, Legend } from "@/components/ui/Fieldset";
import { Input } from "@/components/ui/Input";
import { Listbox, ListboxLabel, ListboxOption } from "@/components/ui/Listbox";
import { useCommanderOptions } from "@/hooks/useCommanderName";

type SideOption = { value: ReportsFilterSide; label: string };

const sideOptions: SideOption[] = [
  { value: "none", label: "None" },
  { value: "sender", label: "Sender" },
  { value: "opponent", label: "Opponent" },
  { value: "both", label: "Either side" },
];

function selectionHasSide(selection: ReportsFilterSide, side: "sender" | "opponent") {
  return selection === "both" || selection === side;
}

function selectionsOverlap(a: ReportsFilterSide, b: ReportsFilterSide) {
  return (
    (selectionHasSide(a, "sender") && selectionHasSide(b, "sender")) ||
    (selectionHasSide(a, "opponent") && selectionHasSide(b, "opponent"))
  );
}

function parseNumberInput(value: string) {
  const trimmed = value.trim();
  if (trimmed === "") {
    return undefined;
  }
  const numeric = Number(trimmed);
  return Number.isFinite(numeric) ? numeric : undefined;
}

type ReportsFilterDialogProps = React.ComponentPropsWithoutRef<typeof Button> & {
  lockedPlayerId?: number;
};

export function ReportsFilterDialog({ lockedPlayerId, ...props }: ReportsFilterDialogProps) {
  const context = useContext(ReportsFilterContext);
  if (!context) throw new Error("ReportsFilterDialog must be used within a ReportsFilterProvider");

  const {
    playerId,
    setPlayerId,
    type,
    setType,
    senderPrimaryCommanderId,
    setSenderPrimaryCommanderId,
    senderSecondaryCommanderId,
    setSenderSecondaryCommanderId,
    opponentPrimaryCommanderId,
    setOpponentPrimaryCommanderId,
    opponentSecondaryCommanderId,
    setOpponentSecondaryCommanderId,
    rallySide,
    setRallySide,
    garrisonSide,
    setGarrisonSide,
    garrisonBuildingType,
    setGarrisonBuildingType,
    reset,
  } = context;

  const [isOpen, setIsOpen] = useState(false);
  const hasLockedPlayerId = typeof lockedPlayerId === "number" && Number.isFinite(lockedPlayerId);
  const [localPlayerId, setLocalPlayerId] = useState(() => {
    const initialId = hasLockedPlayerId ? lockedPlayerId : playerId;
    return typeof initialId === "number" ? String(initialId) : "";
  });
  const [localType, setLocalType] = useState<ReportsFilterType | "">(() => type ?? "");
  const [localSenderPrimaryCommanderId, setLocalSenderPrimaryCommanderId] = useState(() =>
    typeof senderPrimaryCommanderId === "number" ? String(senderPrimaryCommanderId) : ""
  );
  const [localSenderSecondaryCommanderId, setLocalSenderSecondaryCommanderId] = useState(() =>
    typeof senderSecondaryCommanderId === "number" ? String(senderSecondaryCommanderId) : ""
  );
  const [localOpponentPrimaryCommanderId, setLocalOpponentPrimaryCommanderId] = useState(() =>
    typeof opponentPrimaryCommanderId === "number" ? String(opponentPrimaryCommanderId) : ""
  );
  const [localOpponentSecondaryCommanderId, setLocalOpponentSecondaryCommanderId] = useState(() =>
    typeof opponentSecondaryCommanderId === "number" ? String(opponentSecondaryCommanderId) : ""
  );
  const [localRallySide, setLocalRallySide] = useState<ReportsFilterSide>(() => rallySide);
  const [localGarrisonSide, setLocalGarrisonSide] = useState<ReportsFilterSide>(() => garrisonSide);
  const [localGarrisonBuildingType, setLocalGarrisonBuildingType] = useState<
    ReportsGarrisonBuildingType | ""
  >(() => garrisonBuildingType ?? "");

  const commanderOptions = useCommanderOptions();

  useEffect(() => {
    if (!isOpen) return;
    const nextRallySide = rallySide;
    const nextGarrisonSide = selectionsOverlap(nextRallySide, garrisonSide) ? "none" : garrisonSide;
    const resolvedPlayerId = hasLockedPlayerId ? lockedPlayerId : playerId;
    setLocalPlayerId(typeof resolvedPlayerId === "number" ? String(resolvedPlayerId) : "");
    setLocalType(type ?? "");
    setLocalSenderPrimaryCommanderId(
      typeof senderPrimaryCommanderId === "number" ? String(senderPrimaryCommanderId) : ""
    );
    setLocalSenderSecondaryCommanderId(
      typeof senderSecondaryCommanderId === "number" ? String(senderSecondaryCommanderId) : ""
    );
    setLocalOpponentPrimaryCommanderId(
      typeof opponentPrimaryCommanderId === "number" ? String(opponentPrimaryCommanderId) : ""
    );
    setLocalOpponentSecondaryCommanderId(
      typeof opponentSecondaryCommanderId === "number" ? String(opponentSecondaryCommanderId) : ""
    );
    setLocalRallySide(nextRallySide);
    setLocalGarrisonSide(nextGarrisonSide);
    setLocalGarrisonBuildingType(nextGarrisonSide === "none" ? "" : (garrisonBuildingType ?? ""));
  }, [
    isOpen,
    playerId,
    type,
    senderPrimaryCommanderId,
    senderSecondaryCommanderId,
    opponentPrimaryCommanderId,
    opponentSecondaryCommanderId,
    rallySide,
    garrisonSide,
    garrisonBuildingType,
    lockedPlayerId,
    hasLockedPlayerId,
  ]);

  const handleApply = () => {
    const nextGarrisonSide = selectionsOverlap(localRallySide, localGarrisonSide)
      ? "none"
      : localGarrisonSide;
    const resolvedPlayerId = hasLockedPlayerId ? lockedPlayerId : parseNumberInput(localPlayerId);
    setPlayerId(
      typeof resolvedPlayerId === "number" && Number.isFinite(resolvedPlayerId)
        ? resolvedPlayerId
        : undefined
    );
    setType(localType === "" ? undefined : localType);
    setSenderPrimaryCommanderId(parseNumberInput(localSenderPrimaryCommanderId));
    setSenderSecondaryCommanderId(parseNumberInput(localSenderSecondaryCommanderId));
    setOpponentPrimaryCommanderId(parseNumberInput(localOpponentPrimaryCommanderId));
    setOpponentSecondaryCommanderId(parseNumberInput(localOpponentSecondaryCommanderId));
    setRallySide(localRallySide);
    setGarrisonSide(nextGarrisonSide);
    setGarrisonBuildingType(
      nextGarrisonSide === "none"
        ? undefined
        : localGarrisonBuildingType === ""
          ? undefined
          : localGarrisonBuildingType
    );
    setIsOpen(false);
  };

  return (
    <>
      <Button type="button" onClick={() => setIsOpen(true)} {...props} />
      <Dialog open={isOpen} onClose={setIsOpen} size="4xl">
        <DialogTitle>Filters</DialogTitle>
        <DialogDescription>
          Filter battle reports by metadata, commanders, and battle roles.
        </DialogDescription>
        <DialogBody>
          <div className="grid gap-6 lg:grid-cols-3">
            <Fieldset>
              <Legend>Metadata</Legend>
              <div data-slot="control" className="space-y-6">
                <Field>
                  <Label>Governor ID</Label>
                  <Input
                    inputMode="numeric"
                    pattern="[0-9]*"
                    placeholder="71738515"
                    value={localPlayerId}
                    disabled={hasLockedPlayerId}
                    onChange={(event) => {
                      if (hasLockedPlayerId) return;
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
                    <ListboxOption value="home">
                      <ListboxLabel>Home</ListboxLabel>
                    </ListboxOption>
                  </Listbox>
                </Field>
              </div>
            </Fieldset>
            <Fieldset>
              <Legend>Sender</Legend>
              <div data-slot="control" className="space-y-6">
                <Field>
                  <Label>Primary Commander</Label>
                  <Listbox
                    value={localSenderPrimaryCommanderId}
                    onChange={(value) => {
                      setLocalSenderPrimaryCommanderId(value);
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
                    value={localSenderSecondaryCommanderId}
                    onChange={(value) => {
                      setLocalSenderSecondaryCommanderId(value);
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
              </div>
            </Fieldset>
            <Fieldset>
              <Legend>Opponent</Legend>
              <div data-slot="control" className="space-y-6">
                <Field>
                  <Label>Primary Commander</Label>
                  <Listbox
                    value={localOpponentPrimaryCommanderId}
                    onChange={(value) => {
                      setLocalOpponentPrimaryCommanderId(value);
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
                    value={localOpponentSecondaryCommanderId}
                    onChange={(value) => {
                      setLocalOpponentSecondaryCommanderId(value);
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
              </div>
            </Fieldset>
            <Fieldset className="lg:col-span-3">
              <Legend>Battle</Legend>
              <div data-slot="control" className="grid gap-6 lg:grid-cols-3">
                <Field>
                  <Label>Rally Side</Label>
                  <Listbox<ReportsFilterSide>
                    value={localRallySide}
                    onChange={(value) => {
                      setLocalRallySide(value);
                      if (selectionsOverlap(value, localGarrisonSide)) {
                        setLocalGarrisonSide("none");
                        setLocalGarrisonBuildingType("");
                      }
                    }}
                  >
                    {sideOptions.map((option) => (
                      <ListboxOption
                        key={option.value}
                        value={option.value}
                        disabled={selectionsOverlap(option.value, localGarrisonSide)}
                      >
                        <ListboxLabel>{option.label}</ListboxLabel>
                      </ListboxOption>
                    ))}
                  </Listbox>
                </Field>
                <Field>
                  <Label>Garrison Side</Label>
                  <Listbox<ReportsFilterSide>
                    value={localGarrisonSide}
                    onChange={(value) => {
                      setLocalGarrisonSide(value);
                      if (selectionsOverlap(localRallySide, value)) {
                        setLocalRallySide("none");
                      }
                      if (value === "none") {
                        setLocalGarrisonBuildingType("");
                      }
                    }}
                  >
                    {sideOptions.map((option) => (
                      <ListboxOption
                        key={option.value}
                        value={option.value}
                        disabled={selectionsOverlap(option.value, localRallySide)}
                      >
                        <ListboxLabel>{option.label}</ListboxLabel>
                      </ListboxOption>
                    ))}
                  </Listbox>
                </Field>
                {localGarrisonSide !== "none" ? (
                  <Field>
                    <Label>Garrison Building</Label>
                    <Listbox<ReportsGarrisonBuildingType | "">
                      value={localGarrisonBuildingType}
                      onChange={(value) => {
                        setLocalGarrisonBuildingType(value);
                      }}
                    >
                      <ListboxOption value="">
                        <ListboxLabel>Any</ListboxLabel>
                      </ListboxOption>
                      <ListboxOption value="flag">
                        <ListboxLabel>Alliance Flag</ListboxLabel>
                      </ListboxOption>
                      <ListboxOption value="fortress">
                        <ListboxLabel>Alliance Fortress</ListboxLabel>
                      </ListboxOption>
                      <ListboxOption value="other">
                        <ListboxLabel>Other</ListboxLabel>
                      </ListboxOption>
                    </Listbox>
                  </Field>
                ) : null}
              </div>
            </Fieldset>
          </div>
        </DialogBody>
        <DialogActions>
          <Button plain onClick={() => setIsOpen(false)}>
            Cancel
          </Button>
          <Button
            plain
            onClick={() => {
              reset();
              if (hasLockedPlayerId) {
                setPlayerId(lockedPlayerId);
              }
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
