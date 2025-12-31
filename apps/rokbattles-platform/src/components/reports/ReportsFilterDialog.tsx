"use client";

import { useTranslations } from "next-intl";
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
  const t = useTranslations("reports.filter");
  const tCommon = useTranslations("common");
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

  const sideOptions: SideOption[] = [
    { value: "none", label: t("sideOptions.none") },
    { value: "sender", label: tCommon("labels.sender") },
    { value: "opponent", label: tCommon("labels.opponent") },
    { value: "both", label: t("sideOptions.either") },
  ];

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
        <DialogTitle>{t("title")}</DialogTitle>
        <DialogDescription>{t("description")}</DialogDescription>
        <DialogBody>
          <div className="grid gap-6 lg:grid-cols-3">
            <Fieldset>
              <Legend>{t("sections.metadata")}</Legend>
              <div data-slot="control" className="space-y-6">
                <Field>
                  <Label>{tCommon("fields.governorId")}</Label>
                  <Input
                    inputMode="numeric"
                    pattern="[0-9]*"
                    placeholder={tCommon("placeholders.governorId")}
                    value={localPlayerId}
                    disabled={hasLockedPlayerId}
                    onChange={(event) => {
                      if (hasLockedPlayerId) return;
                      setLocalPlayerId(event.target.value);
                    }}
                  />
                </Field>
                <Field>
                  <Label>{t("fields.type")}</Label>
                  <Listbox<ReportsFilterType | "">
                    value={localType}
                    onChange={(value) => {
                      setLocalType(value);
                    }}
                  >
                    <ListboxOption value="">
                      <ListboxLabel>{tCommon("labels.all")}</ListboxLabel>
                    </ListboxOption>
                    <ListboxOption value="kvk">
                      <ListboxLabel>{t("typeOptions.kvk")}</ListboxLabel>
                    </ListboxOption>
                    <ListboxOption value="ark">
                      <ListboxLabel>{t("typeOptions.ark")}</ListboxLabel>
                    </ListboxOption>
                    <ListboxOption value="home">
                      <ListboxLabel>{t("typeOptions.home")}</ListboxLabel>
                    </ListboxOption>
                  </Listbox>
                </Field>
              </div>
            </Fieldset>
            <Fieldset>
              <Legend>{tCommon("labels.sender")}</Legend>
              <div data-slot="control" className="space-y-6">
                <Field>
                  <Label>{t("fields.primaryCommander")}</Label>
                  <Listbox
                    value={localSenderPrimaryCommanderId}
                    onChange={(value) => {
                      setLocalSenderPrimaryCommanderId(value);
                    }}
                  >
                    <ListboxOption value="">
                      <ListboxLabel>{tCommon("labels.all")}</ListboxLabel>
                    </ListboxOption>
                    {commanderOptions.map((option) => (
                      <ListboxOption key={option.id} value={String(option.id)}>
                        <ListboxLabel>{option.name}</ListboxLabel>
                      </ListboxOption>
                    ))}
                  </Listbox>
                </Field>
                <Field>
                  <Label>{t("fields.secondaryCommander")}</Label>
                  <Listbox
                    value={localSenderSecondaryCommanderId}
                    onChange={(value) => {
                      setLocalSenderSecondaryCommanderId(value);
                    }}
                  >
                    <ListboxOption value="">
                      <ListboxLabel>{tCommon("labels.all")}</ListboxLabel>
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
              <Legend>{tCommon("labels.opponent")}</Legend>
              <div data-slot="control" className="space-y-6">
                <Field>
                  <Label>{t("fields.primaryCommander")}</Label>
                  <Listbox
                    value={localOpponentPrimaryCommanderId}
                    onChange={(value) => {
                      setLocalOpponentPrimaryCommanderId(value);
                    }}
                  >
                    <ListboxOption value="">
                      <ListboxLabel>{tCommon("labels.all")}</ListboxLabel>
                    </ListboxOption>
                    {commanderOptions.map((option) => (
                      <ListboxOption key={option.id} value={String(option.id)}>
                        <ListboxLabel>{option.name}</ListboxLabel>
                      </ListboxOption>
                    ))}
                  </Listbox>
                </Field>
                <Field>
                  <Label>{t("fields.secondaryCommander")}</Label>
                  <Listbox
                    value={localOpponentSecondaryCommanderId}
                    onChange={(value) => {
                      setLocalOpponentSecondaryCommanderId(value);
                    }}
                  >
                    <ListboxOption value="">
                      <ListboxLabel>{tCommon("labels.all")}</ListboxLabel>
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
              <Legend>{t("sections.battle")}</Legend>
              <div data-slot="control" className="grid gap-6 lg:grid-cols-3">
                <Field>
                  <Label>{t("fields.rallySide")}</Label>
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
                  <Label>{t("fields.garrisonSide")}</Label>
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
                    <Label>{t("fields.garrisonBuilding")}</Label>
                    <Listbox<ReportsGarrisonBuildingType | "">
                      value={localGarrisonBuildingType}
                      onChange={(value) => {
                        setLocalGarrisonBuildingType(value);
                      }}
                    >
                      <ListboxOption value="">
                        <ListboxLabel>{tCommon("labels.any")}</ListboxLabel>
                      </ListboxOption>
                      <ListboxOption value="flag">
                        <ListboxLabel>{t("garrisonBuildingOptions.flag")}</ListboxLabel>
                      </ListboxOption>
                      <ListboxOption value="fortress">
                        <ListboxLabel>{t("garrisonBuildingOptions.fortress")}</ListboxLabel>
                      </ListboxOption>
                      <ListboxOption value="other">
                        <ListboxLabel>{t("garrisonBuildingOptions.other")}</ListboxLabel>
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
            {tCommon("actions.cancel")}
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
            {tCommon("actions.reset")}
          </Button>
          <Button onClick={handleApply}>{tCommon("actions.apply")}</Button>
        </DialogActions>
      </Dialog>
    </>
  );
}
