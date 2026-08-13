"use client";

import {
  ArrowsRightLeftIcon,
  CheckIcon,
  MagnifyingGlassIcon,
  XMarkIcon,
} from "@heroicons/react/16/solid";
import { cn } from "cnfast";
import { usePathname, useRouter, useSearchParams } from "next/navigation";
import { useExtracted, useLocale } from "next-intl";
import { useDeferredValue, useMemo, useState } from "react";
import { CommanderIcon } from "@/components/commander-icon";
import { Button } from "@/components/ui/button";
import {
  Drawer,
  DrawerActions,
  DrawerBody,
  DrawerDescription,
  DrawerTitle,
} from "@/components/ui/drawer";
import { Input, InputGroup } from "@/components/ui/input";
import { Listbox, ListboxLabel, ListboxOption } from "@/components/ui/listbox";
import type { CombatLabCommanderOption } from "@/lib/combat-lab/commanders";

type PairingSlot = "primary" | "secondary";

type CommanderPairingDrawerProps = {
  commanderOptions: CombatLabCommanderOption[];
  onClose: () => void;
  open: boolean;
  primaryCommanderId: number;
  secondaryCommanderId: number;
};

export function CommanderPairingDrawer({
  commanderOptions,
  onClose,
  open,
  primaryCommanderId,
  secondaryCommanderId,
}: CommanderPairingDrawerProps) {
  const t = useExtracted();
  const locale = useLocale();
  const pathname = usePathname();
  const router = useRouter();
  const searchParams = useSearchParams();
  const [activeSlot, setActiveSlot] = useState<PairingSlot>("primary");
  const [draftPrimaryId, setDraftPrimaryId] = useState(primaryCommanderId);
  const [draftSecondaryId, setDraftSecondaryId] = useState(secondaryCommanderId);
  const [query, setQuery] = useState("");
  const [selectedTalents, setSelectedTalents] = useState<string[]>([]);
  const deferredQuery = useDeferredValue(query);
  const numberFormatter = useMemo(() => new Intl.NumberFormat(locale), [locale]);

  const commanderById = useMemo(
    () => new Map(commanderOptions.map((commander) => [commander.id, commander])),
    [commanderOptions]
  );
  const talentOptions = useMemo(
    () =>
      Array.from(new Set(commanderOptions.flatMap((commander) => commander.talents))).sort(
        (left, right) => talentLabel(left).localeCompare(talentLabel(right), locale)
      ),
    [commanderOptions, locale]
  );
  const filteredCommanders = useMemo(() => {
    const normalizedQuery = deferredQuery.trim().toLocaleLowerCase(locale);

    return commanderOptions.filter((commander) => {
      const matchesName =
        normalizedQuery.length === 0 ||
        commander.name.toLocaleLowerCase(locale).includes(normalizedQuery);
      const matchesTalents = selectedTalents.every((talent) => commander.talents.includes(talent));
      return matchesName && matchesTalents;
    });
  }, [commanderOptions, deferredQuery, locale, selectedTalents]);

  const primaryCommander = commanderById.get(draftPrimaryId);
  const secondaryCommander = commanderById.get(draftSecondaryId);
  const oppositeCommanderId = activeSlot === "primary" ? draftSecondaryId : draftPrimaryId;
  const selectedCommanderId = activeSlot === "primary" ? draftPrimaryId : draftSecondaryId;
  const hasFilters = query.length > 0 || selectedTalents.length > 0;
  const pairingIsValid =
    Boolean(primaryCommander) && Boolean(secondaryCommander) && draftPrimaryId !== draftSecondaryId;

  function handleOpenChange(nextOpen: boolean) {
    if (nextOpen) {
      return;
    }
    resetDraft();
    onClose();
  }

  function resetDraft() {
    setDraftPrimaryId(primaryCommanderId);
    setDraftSecondaryId(secondaryCommanderId);
    setActiveSlot("primary");
    setQuery("");
    setSelectedTalents([]);
  }

  function closeDrawer() {
    handleOpenChange(false);
  }

  function selectCommander(commanderId: number) {
    if (commanderId === oppositeCommanderId) {
      return;
    }

    if (activeSlot === "primary") {
      setDraftPrimaryId(commanderId);
      setActiveSlot("secondary");
    } else {
      setDraftSecondaryId(commanderId);
    }
  }

  function applyPairing() {
    if (!pairingIsValid) {
      return;
    }

    const nextSearchParams = new URLSearchParams(searchParams.toString());
    nextSearchParams.set("primary", String(draftPrimaryId));
    nextSearchParams.set("secondary", String(draftSecondaryId));
    onClose();
    router.push(`${pathname}?${nextSearchParams.toString()}`);
  }

  return (
    <Drawer onClose={handleOpenChange} open={open} size="2xl">
      <div className="flex items-start justify-between gap-4">
        <div>
          <DrawerTitle>{t("Change pairing")}</DrawerTitle>
          <DrawerDescription>
            {t("Choose two different commanders and click Save to load their data.")}
          </DrawerDescription>
        </div>
        <Button aria-label={t("Close pairing picker")} onClick={closeDrawer} plain>
          <XMarkIcon />
        </Button>
      </div>

      <DrawerBody className="space-y-7">
        <section aria-label={t("Pairing order")}>
          <div className="grid grid-cols-[minmax(0,1fr)_auto_minmax(0,1fr)] items-center gap-2">
            <PairingSlotButton
              active={activeSlot === "primary"}
              commander={primaryCommander}
              label={t("Primary")}
              onClick={() => setActiveSlot("primary")}
            />
            <button
              aria-label={t("Swap primary and secondary commanders")}
              className="rounded-full border border-zinc-950/10 bg-white p-2 text-zinc-500 shadow-sm transition hover:border-zinc-950/20 hover:text-zinc-950 focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-blue-500 dark:border-white/10 dark:bg-white/5 dark:text-zinc-400 dark:hover:border-white/20 dark:hover:text-white"
              onClick={() => {
                setDraftPrimaryId(draftSecondaryId);
                setDraftSecondaryId(draftPrimaryId);
              }}
              type="button"
            >
              <ArrowsRightLeftIcon className="size-4" />
            </button>
            <PairingSlotButton
              active={activeSlot === "secondary"}
              commander={secondaryCommander}
              label={t("Secondary")}
              onClick={() => setActiveSlot("secondary")}
            />
          </div>
        </section>

        <div className="space-y-3">
          <InputGroup>
            <MagnifyingGlassIcon data-slot="icon" />
            <Input
              aria-label={t("Search by commander")}
              onChange={(event) => setQuery(event.target.value)}
              placeholder={t("Search by commander")}
              type="search"
              value={query}
            />
          </InputGroup>
          <Listbox<string>
            aria-label={t("Filter commander categories")}
            multiple
            onChange={setSelectedTalents}
            placeholder={t("Filter commander categories")}
            renderValue={(talents) =>
              Array.isArray(talents) && talents.length > 0 ? (
                talents.map(talentLabel).join(", ")
              ) : (
                <span className="block truncate text-zinc-500">
                  {t("Filter commander categories")}
                </span>
              )
            }
            value={selectedTalents}
          >
            {talentOptions.map((talent) => (
              <ListboxOption key={talent} value={talent}>
                <ListboxLabel>{talentLabel(talent)}</ListboxLabel>
              </ListboxOption>
            ))}
          </Listbox>
        </div>

        <section aria-labelledby="commander-results-heading">
          <div className="mb-3 flex items-end justify-between gap-3">
            <h3
              className="font-semibold text-sm/6 text-zinc-950 dark:text-white"
              id="commander-results-heading"
            >
              {activeSlot === "primary"
                ? t("Choose the primary commander")
                : t("Choose the secondary commander")}
            </h3>
            <div className="flex shrink-0 items-center gap-3">
              {hasFilters ? (
                <button
                  className="font-medium text-blue-600 text-xs hover:text-blue-700 focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-blue-500 dark:text-blue-400 dark:hover:text-blue-300"
                  onClick={() => {
                    setQuery("");
                    setSelectedTalents([]);
                  }}
                  type="button"
                >
                  {t("Clear filters")}
                </button>
              ) : null}
              <p aria-live="polite" className="text-xs text-zinc-500 dark:text-zinc-400">
                {numberFormatter.format(filteredCommanders.length)}{" "}
                {filteredCommanders.length === 1 ? t("commander") : t("commanders")}
              </p>
            </div>
          </div>

          {filteredCommanders.length > 0 ? (
            <div className="grid gap-2 sm:grid-cols-2" data-testid="commander-results">
              {filteredCommanders.map((commander) => {
                const selected = commander.id === selectedCommanderId;
                const unavailable = commander.id === oppositeCommanderId;
                return (
                  <button
                    aria-pressed={selected}
                    className={cn(
                      "group flex min-w-0 items-center gap-3 rounded-md border p-2.5 text-left transition [contain-intrinsic-size:auto_68px] [content-visibility:auto] focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-blue-400/70",
                      selected
                        ? "border-blue-400/60 bg-blue-50/60 dark:border-blue-400/50 dark:bg-blue-500/10"
                        : "border-zinc-950/10 hover:border-zinc-950/20 hover:bg-zinc-950/2.5 dark:border-white/10 dark:hover:border-white/20 dark:hover:bg-white/5",
                      unavailable && "cursor-not-allowed opacity-40"
                    )}
                    disabled={unavailable}
                    key={commander.id}
                    onClick={() => selectCommander(commander.id)}
                    type="button"
                  >
                    <CommanderIcon
                      awakened
                      alt={commander.name}
                      className="size-11"
                      id={commander.id}
                      sizes="44px"
                    />
                    <span className="min-w-0 flex-1">
                      <span className="block truncate font-semibold text-sm text-zinc-950 dark:text-white">
                        {commander.name}
                      </span>
                      <span className="mt-0.5 block truncate text-xs text-zinc-500 dark:text-zinc-400">
                        {commander.talents.map(talentLabel).join(" · ")}
                      </span>
                    </span>
                    {selected ? (
                      <CheckIcon className="size-4 shrink-0 text-blue-600 dark:text-blue-400" />
                    ) : null}
                  </button>
                );
              })}
            </div>
          ) : (
            <div
              className="rounded-md border border-dashed border-zinc-950/15 px-5 py-10 text-center dark:border-white/15"
              data-testid="commander-results-empty"
            >
              <p className="font-semibold text-sm text-zinc-950 dark:text-white">
                {t("No legendary commanders match")}
              </p>
              <p className="mt-1 text-sm/6 text-zinc-500 dark:text-zinc-400">
                {t("Try removing a talent or changing your search.")}
              </p>
            </div>
          )}
        </section>
      </DrawerBody>

      <DrawerActions>
        <Button onClick={closeDrawer} plain>
          {t("Cancel")}
        </Button>
        <Button color="blue" disabled={!pairingIsValid} onClick={applyPairing}>
          {t("Save")}
        </Button>
      </DrawerActions>
    </Drawer>
  );
}

function PairingSlotButton({
  active,
  commander,
  label,
  onClick,
}: {
  active: boolean;
  commander: CombatLabCommanderOption | undefined;
  label: string;
  onClick: () => void;
}) {
  return (
    <button
      aria-pressed={active}
      className={cn(
        "min-w-0 rounded-md border p-2.5 text-left transition focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-blue-400/70",
        active
          ? "border-blue-400/60 bg-blue-50/60 dark:border-blue-400/50 dark:bg-blue-500/10"
          : "border-zinc-950/10 hover:border-zinc-950/20 dark:border-white/10 dark:hover:border-white/20"
      )}
      onClick={onClick}
      type="button"
    >
      <span className="block font-semibold text-[0.65rem] uppercase tracking-wider text-zinc-500 dark:text-zinc-400">
        {label}
      </span>
      <span className="mt-1 flex min-w-0 items-center gap-2">
        {commander ? (
          <CommanderIcon
            awakened
            alt={commander.name}
            className="size-8"
            id={commander.id}
            sizes="32px"
          />
        ) : null}
        <span className="truncate font-semibold text-sm text-zinc-950 dark:text-white">
          {commander?.name ?? "—"}
        </span>
      </span>
    </button>
  );
}

function talentLabel(talent: string) {
  return talent.charAt(0).toUpperCase() + talent.slice(1);
}
