"use client";

import { useExtracted } from "next-intl";
import type React from "react";
import { use, useEffect, useState } from "react";
import { Button } from "@/components/ui/button";
import {
  Dialog,
  DialogActions,
  DialogBody,
  DialogDescription,
  DialogTitle,
} from "@/components/ui/dialog";
import { Field, Fieldset, Label, Legend } from "@/components/ui/fieldset";
import { Input } from "@/components/ui/input";
import { Listbox, ListboxLabel, ListboxOption } from "@/components/ui/listbox";
import { GameTranslate } from "@/components/v1/game-translate";
import { useCommanderOptions } from "@/hooks/use-commander-name";
import {
  ReportsFilterContext,
  type ReportsFilterSide,
  type ReportsFilterSubtype,
  type ReportsFilterType,
  type ReportsGarrisonBuildingType,
} from "@/providers/reports-filter-context";

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
  const t = useExtracted();
  const context = use(ReportsFilterContext);
  if (!context) throw new Error("ReportsFilterDialog must be used within a ReportsFilterProvider");

  const {
    playerId,
    setPlayerId,
    type,
    setType,
    subtype,
    setSubtype,
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
  const [localSubtype, setLocalSubtype] = useState<ReportsFilterSubtype | "">(() => subtype ?? "");
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
    { value: "none", label: t("None") },
    { value: "sender", label: t("Sender") },
    { value: "opponent", label: t("Opponent") },
    { value: "both", label: t("Either side") },
  ];

  const commanderOptions = useCommanderOptions();

  useEffect(() => {
    if (!isOpen) return;
    const nextRallySide = rallySide;
    const nextGarrisonSide = selectionsOverlap(nextRallySide, garrisonSide) ? "none" : garrisonSide;
    const resolvedPlayerId = hasLockedPlayerId ? lockedPlayerId : playerId;
    setLocalPlayerId(typeof resolvedPlayerId === "number" ? String(resolvedPlayerId) : "");
    setLocalType(type ?? "");
    setLocalSubtype(subtype ?? "");
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
    subtype,
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
    setSubtype(localSubtype === "" ? undefined : localSubtype);
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
        <DialogTitle>{t("Filters")}</DialogTitle>
        <DialogDescription>
          {t("Filter battle reports by governor, commanders, and battle roles.")}
        </DialogDescription>
        <DialogBody>
          <div className="grid gap-6 lg:grid-cols-3">
            <Fieldset>
              <Legend>{t("Governor")}</Legend>
              <div data-slot="control" className="space-y-6">
                <Field>
                  <Label>{t("Governor ID")}</Label>
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
              </div>
            </Fieldset>
            <Fieldset>
              <Legend>{t("Sender")}</Legend>
              <div data-slot="control" className="space-y-6">
                <Field>
                  <Label>
                    <GameTranslate value="LC_HERO_CHIEFCOMMANDER" />
                  </Label>
                  <Listbox
                    value={localSenderPrimaryCommanderId}
                    onChange={(value) => {
                      setLocalSenderPrimaryCommanderId(value);
                    }}
                  >
                    <ListboxOption value="">
                      <ListboxLabel>{t("Any")}</ListboxLabel>
                    </ListboxOption>
                    {commanderOptions.map((option) => (
                      <ListboxOption key={option.id} value={String(option.id)}>
                        <ListboxLabel>{option.name}</ListboxLabel>
                      </ListboxOption>
                    ))}
                  </Listbox>
                </Field>
                <Field>
                  <Label>
                    <GameTranslate value="LC_HERO_SECONDARYCOMMANDER" />
                  </Label>
                  <Listbox
                    value={localSenderSecondaryCommanderId}
                    onChange={(value) => {
                      setLocalSenderSecondaryCommanderId(value);
                    }}
                  >
                    <ListboxOption value="">
                      <ListboxLabel>{t("Any")}</ListboxLabel>
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
              <Legend>{t("Opponent")}</Legend>
              <div data-slot="control" className="space-y-6">
                <Field>
                  <Label>
                    <GameTranslate value="LC_HERO_CHIEFCOMMANDER" />
                  </Label>
                  <Listbox
                    value={localOpponentPrimaryCommanderId}
                    onChange={(value) => {
                      setLocalOpponentPrimaryCommanderId(value);
                    }}
                  >
                    <ListboxOption value="">
                      <ListboxLabel>{t("Any")}</ListboxLabel>
                    </ListboxOption>
                    {commanderOptions.map((option) => (
                      <ListboxOption key={option.id} value={String(option.id)}>
                        <ListboxLabel>{option.name}</ListboxLabel>
                      </ListboxOption>
                    ))}
                  </Listbox>
                </Field>
                <Field>
                  <Label>
                    <GameTranslate value="LC_HERO_SECONDARYCOMMANDER" />
                  </Label>
                  <Listbox
                    value={localOpponentSecondaryCommanderId}
                    onChange={(value) => {
                      setLocalOpponentSecondaryCommanderId(value);
                    }}
                  >
                    <ListboxOption value="">
                      <ListboxLabel>{t("Any")}</ListboxLabel>
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
              <Legend>{t("Battle")}</Legend>
              <div data-slot="control" className="grid gap-6 lg:grid-cols-3">
                <div className="grid gap-6 lg:col-span-3 lg:grid-cols-3">
                  <Field>
                    <Label>{t("Type")}</Label>
                    <Listbox<ReportsFilterType | "">
                      value={localType}
                      onChange={(value) => {
                        setLocalType(value);
                        setLocalSubtype("");
                      }}
                    >
                      <ListboxOption value="">
                        <ListboxLabel>{t("Any")}</ListboxLabel>
                      </ListboxOption>
                      <ListboxOption value="home">
                        <ListboxLabel>{t("Home")}</ListboxLabel>
                      </ListboxOption>
                      <ListboxOption value="ark">
                        <ListboxLabel>{t("Ark of Osiris/Osiris League")}</ListboxLabel>
                      </ListboxOption>
                      <ListboxOption value="kvk">
                        <ListboxLabel>
                          <GameTranslate value="LC_COMMON_DIC_US_NAME_1" />
                        </ListboxLabel>
                      </ListboxOption>
                      <ListboxOption value="strife">
                        <ListboxLabel>
                          <GameTranslate value="LC_TITAN_TITLE" />
                        </ListboxLabel>
                      </ListboxOption>
                    </Listbox>
                  </Field>
                  {localType === "kvk" || localType === "ark" ? (
                    <Field>
                      <Label>{t("Subtype")}</Label>
                      <Listbox<ReportsFilterSubtype | "">
                        value={localSubtype}
                        onChange={setLocalSubtype}
                      >
                        <ListboxOption value="">
                          <ListboxLabel>{t("Any")}</ListboxLabel>
                        </ListboxOption>
                        {localType === "kvk" ? (
                          <>
                            <ListboxOption value="1">
                              <ListboxLabel>
                                <GameTranslate value="LC_MAP_LAND_SEASON_1" />
                              </ListboxLabel>
                            </ListboxOption>
                            <ListboxOption value="2">
                              <ListboxLabel>
                                <GameTranslate value="LC_MAP_LAND_SEASON_2" />
                              </ListboxLabel>
                            </ListboxOption>
                            <ListboxOption value="3">
                              <ListboxLabel>
                                <GameTranslate value="LC_MAP_LAND_SEASON_3" />
                              </ListboxLabel>
                            </ListboxOption>
                            <ListboxOption value="100">
                              <ListboxLabel>
                                <GameTranslate value="LC_MAP_LAND_SEASON_CONQUER" />
                              </ListboxLabel>
                            </ListboxOption>
                          </>
                        ) : (
                          <>
                            <ListboxOption value="1">
                              <ListboxLabel>
                                <GameTranslate value="LC_BATTLEFIELD_LEAGUEB_GBATTLE" />
                              </ListboxLabel>
                            </ListboxOption>
                            <ListboxOption value="6">
                              <ListboxLabel>
                                <GameTranslate value="LC_BATTLEFIELD_LEAGUEB_SBATTLE" />
                              </ListboxLabel>
                            </ListboxOption>
                            <ListboxOption value="3">
                              <ListboxLabel>
                                <GameTranslate value="LC_BATTLEFIELD_LEAGUE_ENTRANCE" />
                              </ListboxLabel>
                            </ListboxOption>
                            <ListboxOption value="2">
                              <ListboxLabel>
                                <GameTranslate value="LC_BATTLEFIELD_PRACTISE_ENTRY" />
                              </ListboxLabel>
                            </ListboxOption>
                            <ListboxOption value="5">
                              <ListboxLabel>
                                <GameTranslate value="LC_BATTLEFIELD_DIY_BUTTON_01" />
                              </ListboxLabel>
                            </ListboxOption>
                          </>
                        )}
                      </Listbox>
                    </Field>
                  ) : null}
                </div>
                <Field>
                  <Label>{t("Rally")}</Label>
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
                  <Label>{t("Garrison")}</Label>
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
                    <Label>{t("Garrison Building")}</Label>
                    <Listbox<ReportsGarrisonBuildingType | "">
                      value={localGarrisonBuildingType}
                      onChange={(value) => {
                        setLocalGarrisonBuildingType(value);
                      }}
                    >
                      <ListboxOption value="">
                        <ListboxLabel>{t("Any")}</ListboxLabel>
                      </ListboxOption>
                      <ListboxOption value="flag">
                        <ListboxLabel>
                          <GameTranslate value="LC_COMMON_ALLIANCE_FLAG" />
                        </ListboxLabel>
                      </ListboxOption>
                      <ListboxOption value="fortress">
                        <ListboxLabel>
                          <GameTranslate value="LC_ALLIANCE_TERRITORY_BUILD_NAME2" />
                        </ListboxLabel>
                      </ListboxOption>
                      <ListboxOption value="other">
                        <ListboxLabel>{t("Other")}</ListboxLabel>
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
            {t("Cancel")}
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
            {t("Reset")}
          </Button>
          <Button onClick={handleApply}>{t("Apply")}</Button>
        </DialogActions>
      </Dialog>
    </>
  );
}
