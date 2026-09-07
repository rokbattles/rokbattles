"use client";

import { PencilIcon, TrashIcon, XMarkIcon } from "@heroicons/react/20/solid";
import { useExtracted } from "next-intl";
import { useState } from "react";
import { Button } from "@/components/ui/button";
import { Checkbox, CheckboxField } from "@/components/ui/checkbox";
import { Drawer, DrawerBody, DrawerTitle } from "@/components/ui/drawer";
import { Field, Label } from "@/components/ui/fieldset";
import { Input } from "@/components/ui/input";
import { LOST_KINGDOM_TERRITORY_COLORS } from "@/lib/territory/presentation";
import type { Alliance } from "@/lib/territory/types";

type PlannerSettingsDrawerProps = {
  alliances: Alliance[];
  open: boolean;
  showBoundary: boolean;
  showCaves: boolean;
  showResources: boolean;
  showVillages: boolean;
  onAddAlliance: (name: string, color: string) => void;
  onChangeAlliance: (allianceId: string, change: Pick<Alliance, "name" | "color">) => void;
  onClose: () => void;
  onDeleteAlliance: (allianceId: string) => void;
  onShowBoundaryChange: (checked: boolean) => void;
  onShowCavesChange: (checked: boolean) => void;
  onShowResourcesChange: (checked: boolean) => void;
  onShowVillagesChange: (checked: boolean) => void;
};

function suggestedColor(allianceCount: number): string {
  return LOST_KINGDOM_TERRITORY_COLORS[allianceCount % LOST_KINGDOM_TERRITORY_COLORS.length].value;
}

export function PlannerSettingsDrawer({
  alliances,
  open,
  showBoundary,
  showCaves,
  showResources,
  showVillages,
  onAddAlliance,
  onChangeAlliance,
  onClose,
  onDeleteAlliance,
  onShowBoundaryChange,
  onShowCavesChange,
  onShowResourcesChange,
  onShowVillagesChange,
}: PlannerSettingsDrawerProps) {
  const t = useExtracted();
  const colorLabels: Record<string, string> = {
    Red: t("Red"),
    Orange: t("Orange"),
    Yellow: t("Yellow"),
    Green: t("Green"),
    Cyan: t("Cyan"),
    Blue: t("Blue"),
    Purple: t("Purple"),
    Pink: t("Pink"),
  };
  const [editingAllianceId, setEditingAllianceId] = useState<string | null>(null);
  const [name, setName] = useState("");
  const [color, setColor] = useState(() => suggestedColor(alliances.length));

  const clearForm = (allianceCount = alliances.length) => {
    setEditingAllianceId(null);
    setName("");
    setColor(suggestedColor(allianceCount));
  };

  const close = () => {
    clearForm();
    onClose();
  };

  return (
    <Drawer onClose={close} open={open} size="md">
      <div className="flex items-start justify-between gap-4">
        <DrawerTitle>{t("Settings")}</DrawerTitle>
        <Button aria-label={t("Close settings")} className="rounded-md" onClick={close} plain>
          <XMarkIcon />
        </Button>
      </div>

      <DrawerBody>
        <div className="space-y-4">
          <CheckboxField>
            <Checkbox checked={showBoundary} color="blue" onChange={onShowBoundaryChange} />
            <Label>{t("Collision overlay")}</Label>
          </CheckboxField>
          <CheckboxField>
            <Checkbox checked={showResources} color="blue" onChange={onShowResourcesChange} />
            <Label>{t("Resource overlay")}</Label>
          </CheckboxField>
          <CheckboxField>
            <Checkbox checked={showVillages} color="blue" onChange={onShowVillagesChange} />
            <Label>{t("Show villages")}</Label>
          </CheckboxField>
          <CheckboxField>
            <Checkbox checked={showCaves} color="blue" onChange={onShowCavesChange} />
            <Label>{t("Show caves")}</Label>
          </CheckboxField>
        </div>

        <section
          aria-labelledby="planner-alliance-settings"
          className="mt-6 border-zinc-950/10 border-t pt-6 dark:border-white/10"
        >
          <h2
            className="font-semibold text-sm/6 text-zinc-950 dark:text-white"
            id="planner-alliance-settings"
          >
            {t("Alliances")}
          </h2>

          <ul className="mt-3 divide-y divide-zinc-950/10 dark:divide-white/10">
            {alliances.map((alliance) => (
              <li className="flex min-h-12 items-center gap-3 py-2" key={alliance.id}>
                <span
                  aria-label={t("{name} territory color", { name: alliance.name })}
                  className="size-4 shrink-0 rounded-sm border border-black/15 dark:border-white/20"
                  role="img"
                  style={{ backgroundColor: alliance.color }}
                />
                <span className="min-w-0 flex-1 truncate text-sm text-zinc-950 dark:text-white">
                  {alliance.name}
                </span>
                <Button
                  aria-label={t("Edit {name}", { name: alliance.name })}
                  className="rounded-md"
                  onClick={() => {
                    setEditingAllianceId(alliance.id);
                    setName(alliance.name);
                    setColor(alliance.color);
                  }}
                  plain
                >
                  <PencilIcon />
                </Button>
                <Button
                  aria-label={t("Delete {name}", { name: alliance.name })}
                  className="rounded-md"
                  disabled={alliances.length === 1}
                  onClick={() => {
                    onDeleteAlliance(alliance.id);
                    if (editingAllianceId === alliance.id) clearForm(alliances.length - 1);
                  }}
                  plain
                >
                  <TrashIcon />
                </Button>
              </li>
            ))}
          </ul>

          <form
            className="mt-6 border-zinc-950/10 border-t pt-6 dark:border-white/10"
            onSubmit={(event) => {
              event.preventDefault();
              const trimmedName = name.trim();
              if (!trimmedName) return;
              if (editingAllianceId) {
                onChangeAlliance(editingAllianceId, { name: trimmedName, color });
                clearForm();
              } else {
                onAddAlliance(trimmedName, color);
                clearForm(alliances.length + 1);
              }
            }}
          >
            <Field>
              <Label htmlFor="planner-alliance-tag">{t("Alliance tag")}</Label>
              <Input
                className="mt-2 [&_input]:rounded-md"
                id="planner-alliance-tag"
                placeholder={t("Alliance tag")}
                maxLength={4}
                value={name}
                onChange={(event) => setName(event.target.value)}
              />
            </Field>

            <fieldset className="mt-5">
              <legend className="font-medium text-sm/6 text-zinc-950 dark:text-white">
                {t("Territory color")}
              </legend>
              <div className="mt-3 grid grid-cols-4 gap-2">
                {LOST_KINGDOM_TERRITORY_COLORS.map((option) => {
                  const label = colorLabels[option.name] ?? option.name;
                  return (
                    <label className="relative cursor-pointer" key={option.id} title={label}>
                      <input
                        aria-label={t("{color} territory color", { color: label })}
                        checked={color === option.value}
                        className="peer sr-only"
                        name="alliance-territory-color"
                        type="radio"
                        value={option.value}
                        onChange={() => setColor(option.value)}
                      />
                      <span
                        aria-hidden="true"
                        className="flex h-9 items-center justify-center rounded-md border border-black/15 transition peer-checked:ring-2 peer-checked:ring-blue-600 peer-checked:ring-offset-2 peer-focus-visible:outline-2 peer-focus-visible:outline-offset-2 peer-focus-visible:outline-blue-600 dark:border-white/20 dark:peer-checked:ring-offset-zinc-900"
                        style={{ backgroundColor: option.value }}
                      >
                        {color === option.value ? (
                          <span className="size-2.5 rounded-full border-2 border-white shadow-sm" />
                        ) : null}
                      </span>
                    </label>
                  );
                })}
              </div>
            </fieldset>

            <div className="mt-5 flex flex-wrap gap-2">
              <Button className="rounded-md" color="blue" disabled={!name.trim()} type="submit">
                {editingAllianceId ? t("Save") : t("Add")}
              </Button>
              {editingAllianceId ? (
                <Button className="rounded-md" onClick={() => clearForm()} plain type="button">
                  {t("Cancel")}
                </Button>
              ) : null}
            </div>
          </form>
        </section>
      </DrawerBody>
    </Drawer>
  );
}
