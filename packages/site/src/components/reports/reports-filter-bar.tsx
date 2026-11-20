"use client";

import { Plus, X } from "lucide-react";
import { type ReactNode, use, useState } from "react";
import { Button } from "@/components/ui/button";
import { Combobox, ComboboxLabel, ComboboxOption } from "@/components/ui/combobox";
import { Dropdown, DropdownButton, DropdownItem, DropdownMenu } from "@/components/ui/dropdown";
import { Description, Field, Label } from "@/components/ui/fieldset";
import { Input } from "@/components/ui/input";
import { Listbox, ListboxLabel, ListboxOption } from "@/components/ui/listbox";
import { useCommanderOptions } from "@/hooks/use-commander-name";
import {
  ReportsFilterContext,
  type ReportsFilterContextValue,
  type ReportsFilterSide,
  type ReportsGarrisonBuildingType,
  type ReportsTroopType,
} from "@/providers/reports-filter-context";

type FilterKey = "player" | "troop" | "trade" | "date" | "commander" | "rally" | "garrison";

type CommanderOption = { id: number; name: string };

const troopLabels: Record<ReportsTroopType, string> = {
  infantry: "Infantry",
  cavalry: "Cavalry",
  archer: "Archer",
  leadership: "Leadership",
  integration: "Integration",
};

const sideLabels: Record<ReportsFilterSide, string> = {
  none: "None",
  sender: "Sender",
  opponent: "Opponent",
  both: "Either side",
};

const buildingLabels: Record<ReportsGarrisonBuildingType, string> = {
  flag: "Alliance Flag",
  fortress: "Alliance Fortress",
  other: "Other",
};

function hasSide(selection: ReportsFilterSide, side: "sender" | "opponent") {
  return selection === "both" || selection === side;
}

function sidesOverlap(a: ReportsFilterSide, b: ReportsFilterSide) {
  return (
    (hasSide(a, "sender") && hasSide(b, "sender")) ||
    (hasSide(a, "opponent") && hasSide(b, "opponent"))
  );
}

type ReportsFilterBarProps = {
  /** Account pages already pin the report list to one governor, so the player filter is hidden. */
  lockedPlayerId?: number;
  /** Hide the rally/garrison filters on pages that already limit the battle kind. */
  hideRallyGarrison?: boolean;
};

type Chip = { key: FilterKey; label: string; onClear: () => void };

export function ReportsFilterBar({
  lockedPlayerId,
  hideRallyGarrison = false,
}: ReportsFilterBarProps) {
  const context = use(ReportsFilterContext);
  if (!context) throw new Error("ReportsFilterBar must be used within a ReportsFilterProvider");

  const commanders: CommanderOption[] = useCommanderOptions();
  const [editing, setEditing] = useState<FilterKey | null>(null);

  const available: { key: FilterKey; label: string }[] = [
    ...(lockedPlayerId === undefined ? [{ key: "player" as const, label: "Player" }] : []),
    { key: "troop", label: "Troop type" },
    { key: "trade", label: "Trade %" },
    { key: "date", label: "Date range" },
    { key: "commander", label: "Commander" },
    ...(hideRallyGarrison
      ? []
      : [
          { key: "rally" as const, label: "Rally" },
          { key: "garrison" as const, label: "Garrison" },
        ]),
  ];

  const chips = buildChips(context, commanders, available);

  const closeEditor = () => setEditing(null);

  return (
    <div className="mt-4">
      <div className="flex flex-wrap items-center gap-2">
        {chips.map((chip) => (
          <FilterChip
            key={chip.key}
            label={chip.label}
            active={editing === chip.key}
            onEdit={() => setEditing(editing === chip.key ? null : chip.key)}
            onClear={() => {
              chip.onClear();
              if (editing === chip.key) closeEditor();
            }}
          />
        ))}
        <Dropdown>
          <DropdownButton plain className="text-sm">
            <Plus data-slot="icon" aria-hidden="true" />
            Add filter
          </DropdownButton>
          <DropdownMenu anchor="bottom start">
            {available.map((option) => (
              <DropdownItem key={option.key} onClick={() => setEditing(option.key)}>
                {option.label}
              </DropdownItem>
            ))}
          </DropdownMenu>
        </Dropdown>
        {chips.length > 1 ? (
          <Button
            plain
            className="text-sm"
            onClick={() => {
              context.reset();
              closeEditor();
            }}
          >
            Clear all
          </Button>
        ) : null}
      </div>
      {editing ? (
        <div className="mt-3 rounded-xl border border-white/10 bg-white/5 p-4">
          <FilterEditor
            filter={editing}
            context={context}
            commanders={commanders}
            onDone={closeEditor}
          />
        </div>
      ) : null}
    </div>
  );
}

function FilterChip({
  label,
  active,
  onEdit,
  onClear,
}: {
  label: string;
  active: boolean;
  onEdit: () => void;
  onClear: () => void;
}) {
  return (
    <span
      className={`inline-flex items-center rounded-full border text-sm/6 ${
        active ? "border-white/30 bg-white/15" : "border-white/10 bg-white/5"
      }`}
    >
      <button
        type="button"
        onClick={onEdit}
        className="rounded-l-full py-0.5 pr-1.5 pl-3 text-white hover:text-zinc-300"
      >
        {label}
      </button>
      <button
        type="button"
        onClick={onClear}
        aria-label={`Remove filter: ${label}`}
        className="rounded-r-full py-0.5 pr-2 pl-1 text-zinc-400 hover:text-white"
      >
        <X aria-hidden="true" className="size-4" />
      </button>
    </span>
  );
}

function commanderName(commanders: CommanderOption[], id: number | undefined) {
  if (id === undefined) return undefined;
  return commanders.find((commander) => commander.id === id)?.name ?? String(id);
}

function pairLabel(
  title: string,
  commanders: CommanderOption[],
  primary: number | undefined,
  secondary: number | undefined
) {
  const first = commanderName(commanders, primary) ?? "Any";
  const second = commanderName(commanders, secondary);
  return second ? `${title}: ${first} + ${second}` : `${title}: ${first}`;
}

function buildChips(
  c: ReportsFilterContextValue,
  commanders: CommanderOption[],
  available: { key: FilterKey }[]
): Chip[] {
  const offered = new Set(available.map((option) => option.key));
  const chips: Chip[] = [];

  if (offered.has("player") && c.playerId !== undefined) {
    chips.push({
      key: "player",
      label: `Player: ${c.playerId}`,
      onClear: () => c.setPlayerId(undefined),
    });
  }
  if (c.troopType) {
    chips.push({
      key: "troop",
      label: `Troop type: ${troopLabels[c.troopType]}`,
      onClear: () => c.setTroopType(undefined),
    });
  }
  if (c.tradeMin !== undefined || c.tradeMax !== undefined) {
    chips.push({
      key: "trade",
      label: tradeLabel(c.tradeMin, c.tradeMax),
      onClear: () => {
        c.setTradeMin(undefined);
        c.setTradeMax(undefined);
      },
    });
  }
  if (c.dateFrom !== undefined || c.dateTo !== undefined) {
    chips.push({
      key: "date",
      label: dateLabel(c.dateFrom, c.dateTo),
      onClear: () => {
        c.setDateFrom(undefined);
        c.setDateTo(undefined);
      },
    });
  }
  if (c.commanderPrimaryId !== undefined || c.commanderSecondaryId !== undefined) {
    chips.push({
      key: "commander",
      label: pairLabel("Commander", commanders, c.commanderPrimaryId, c.commanderSecondaryId),
      onClear: () => {
        c.setCommanderPrimaryId(undefined);
        c.setCommanderSecondaryId(undefined);
      },
    });
  }
  if (offered.has("rally") && c.rallySide !== "none") {
    chips.push({
      key: "rally",
      label: `Rally: ${sideLabels[c.rallySide]}`,
      onClear: () => c.setRallySide("none"),
    });
  }
  if (offered.has("garrison") && c.garrisonSide !== "none") {
    const building = c.garrisonBuildingType ? ` · ${buildingLabels[c.garrisonBuildingType]}` : "";
    chips.push({
      key: "garrison",
      label: `Garrison: ${sideLabels[c.garrisonSide]}${building}`,
      onClear: () => {
        c.setGarrisonSide("none");
        c.setGarrisonBuildingType(undefined);
      },
    });
  }

  return chips;
}

function FilterEditor({
  filter,
  context: c,
  commanders,
  onDone,
}: {
  filter: FilterKey;
  context: ReportsFilterContextValue;
  commanders: CommanderOption[];
  onDone: () => void;
}) {
  let body: ReactNode;

  if (filter === "player") {
    body = <PlayerEditor value={c.playerId} onChange={c.setPlayerId} />;
  } else if (filter === "troop") {
    body = (
      <Field>
        <Label>Troop type</Label>
        <Listbox<ReportsTroopType | "">
          value={c.troopType ?? ""}
          onChange={(value) => c.setTroopType(value === "" ? undefined : value)}
        >
          <ListboxOption value="">
            <ListboxLabel>Any</ListboxLabel>
          </ListboxOption>
          {(Object.keys(troopLabels) as ReportsTroopType[]).map((troop) => (
            <ListboxOption key={troop} value={troop}>
              <ListboxLabel>{troopLabels[troop]}</ListboxLabel>
            </ListboxOption>
          ))}
        </Listbox>
        <Description>
          Matches battles where the primary commander on either side has this specialty.
        </Description>
      </Field>
    );
  } else if (filter === "trade") {
    body = (
      <TradeEditor
        min={c.tradeMin}
        max={c.tradeMax}
        onChange={(min, max) => {
          c.setTradeMin(min);
          c.setTradeMax(max);
        }}
      />
    );
  } else if (filter === "date") {
    body = (
      <DateEditor
        from={c.dateFrom}
        to={c.dateTo}
        onChange={(from, to) => {
          c.setDateFrom(from);
          c.setDateTo(to);
        }}
      />
    );
  } else if (filter === "commander") {
    body = (
      <CommanderPair
        commanders={commanders}
        primary={c.commanderPrimaryId}
        secondary={c.commanderSecondaryId}
        onPrimary={c.setCommanderPrimaryId}
        onSecondary={c.setCommanderSecondaryId}
      />
    );
  } else if (filter === "rally") {
    body = (
      <SideSelect
        label="Rally"
        value={c.rallySide}
        blockedBy={c.garrisonSide}
        onChange={(value) => {
          c.setRallySide(value);
          if (sidesOverlap(value, c.garrisonSide)) c.setGarrisonSide("none");
        }}
      />
    );
  } else {
    body = (
      <div className="grid gap-4 sm:grid-cols-2">
        <SideSelect
          label="Garrison"
          value={c.garrisonSide}
          blockedBy={c.rallySide}
          onChange={(value) => {
            c.setGarrisonSide(value);
            if (sidesOverlap(c.rallySide, value)) c.setRallySide("none");
            if (value === "none") c.setGarrisonBuildingType(undefined);
          }}
        />
        {c.garrisonSide !== "none" ? (
          <Field>
            <Label>Garrison building</Label>
            <Listbox<ReportsGarrisonBuildingType | "">
              value={c.garrisonBuildingType ?? ""}
              onChange={(value) => c.setGarrisonBuildingType(value === "" ? undefined : value)}
            >
              <ListboxOption value="">
                <ListboxLabel>Any</ListboxLabel>
              </ListboxOption>
              {(Object.keys(buildingLabels) as ReportsGarrisonBuildingType[]).map((type) => (
                <ListboxOption key={type} value={type}>
                  <ListboxLabel>{buildingLabels[type]}</ListboxLabel>
                </ListboxOption>
              ))}
            </Listbox>
          </Field>
        ) : null}
      </div>
    );
  }

  return (
    <div className="flex flex-col gap-4">
      {body}
      <div className="flex justify-end">
        <Button plain onClick={onDone}>
          Done
        </Button>
      </div>
    </div>
  );
}

function SideSelect({
  label,
  value,
  blockedBy,
  onChange,
}: {
  label: string;
  value: ReportsFilterSide;
  blockedBy: ReportsFilterSide;
  onChange: (value: ReportsFilterSide) => void;
}) {
  return (
    <Field>
      <Label>{label}</Label>
      <Listbox<ReportsFilterSide> value={value} onChange={onChange}>
        {(Object.keys(sideLabels) as ReportsFilterSide[]).map((side) => (
          <ListboxOption key={side} value={side} disabled={sidesOverlap(side, blockedBy)}>
            <ListboxLabel>{sideLabels[side]}</ListboxLabel>
          </ListboxOption>
        ))}
      </Listbox>
    </Field>
  );
}

function dateLabel(from: string | undefined, to: string | undefined) {
  if (from && to) return `Date: ${from} to ${to}`;
  if (from) return `Date: from ${from}`;
  return `Date: until ${to}`;
}

function DateEditor({
  from,
  to,
  onChange,
}: {
  from: string | undefined;
  to: string | undefined;
  onChange: (from: string | undefined, to: string | undefined) => void;
}) {
  const update = (nextFrom: string | undefined, nextTo: string | undefined) => {
    // Swap if the dates were picked the wrong way round.
    if (nextFrom && nextTo && nextFrom > nextTo) {
      onChange(nextTo, nextFrom);
      return;
    }
    onChange(nextFrom, nextTo);
  };

  return (
    <div className="flex flex-col gap-3">
      <div className="grid gap-4 sm:grid-cols-2">
        <Field>
          <Label>From (UTC)</Label>
          <Input
            type="date"
            value={from ?? ""}
            onChange={(event) => update(event.target.value || undefined, to)}
          />
        </Field>
        <Field>
          <Label>To (UTC)</Label>
          <Input
            type="date"
            value={to ?? ""}
            onChange={(event) => update(from, event.target.value || undefined)}
          />
        </Field>
      </div>
      <p className="text-sm/6 text-zinc-500">
        Both dates are included. Times are in UTC, like the report list.
      </p>
    </div>
  );
}

function tradeLabel(min: number | undefined, max: number | undefined) {
  if (min !== undefined && max !== undefined) return `Trade %: ${min}–${max}`;
  if (min !== undefined) return `Trade %: ${min} or more`;
  return `Trade %: ${max} or less`;
}

function parseBound(text: string) {
  const trimmed = text.trim();
  if (trimmed === "") return undefined;
  const numeric = Number(trimmed);
  return Number.isInteger(numeric) && numeric >= 0 ? numeric : undefined;
}

function TradeEditor({
  min,
  max,
  onChange,
}: {
  min: number | undefined;
  max: number | undefined;
  onChange: (min: number | undefined, max: number | undefined) => void;
}) {
  const [minText, setMinText] = useState(min === undefined ? "" : String(min));
  const [maxText, setMaxText] = useState(max === undefined ? "" : String(max));

  const commit = () => {
    const nextMin = parseBound(minText);
    const nextMax = parseBound(maxText);
    // Swap if the bounds were entered the wrong way round.
    if (nextMin !== undefined && nextMax !== undefined && nextMin > nextMax) {
      onChange(nextMax, nextMin);
      setMinText(String(nextMax));
      setMaxText(String(nextMin));
      return;
    }
    onChange(nextMin, nextMax);
  };

  const submitOnEnter = (event: React.KeyboardEvent) => {
    if (event.key === "Enter") commit();
  };

  return (
    <div className="grid gap-4 sm:grid-cols-2">
      <Field>
        <Label>Trade % at least</Label>
        <Input
          inputMode="numeric"
          placeholder="e.g. 150"
          value={minText}
          onChange={(event) => setMinText(event.target.value)}
          onBlur={commit}
          onKeyDown={submitOnEnter}
        />
      </Field>
      <Field>
        <Label>Trade % at most</Label>
        <Input
          inputMode="numeric"
          placeholder="e.g. 400"
          value={maxText}
          onChange={(event) => setMaxText(event.target.value)}
          onBlur={commit}
          onKeyDown={submitOnEnter}
        />
      </Field>
    </div>
  );
}

function PlayerEditor({
  value,
  onChange,
}: {
  value: number | undefined;
  onChange: (value: number | undefined) => void;
}) {
  const [text, setText] = useState(value === undefined ? "" : String(value));

  const commit = () => {
    const trimmed = text.trim();
    const numeric = Number(trimmed);
    onChange(trimmed !== "" && Number.isFinite(numeric) ? numeric : undefined);
  };

  return (
    <Field>
      <Label>Player ID</Label>
      <Input
        inputMode="numeric"
        placeholder="Governor ID"
        value={text}
        onChange={(event) => setText(event.target.value)}
        onBlur={commit}
        onKeyDown={(event) => {
          if (event.key === "Enter") commit();
        }}
      />
    </Field>
  );
}

function CommanderPair({
  commanders,
  primary,
  secondary,
  onPrimary,
  onSecondary,
}: {
  commanders: CommanderOption[];
  primary: number | undefined;
  secondary: number | undefined;
  onPrimary: (id: number | undefined) => void;
  onSecondary: (id: number | undefined) => void;
}) {
  return (
    <div className="grid gap-4 sm:grid-cols-2">
      <CommanderSearch
        label="Primary commander"
        commanders={commanders}
        value={primary}
        onChange={onPrimary}
      />
      <CommanderSearch
        label="Secondary commander"
        commanders={commanders}
        value={secondary}
        onChange={onSecondary}
      />
    </div>
  );
}

function CommanderSearch({
  label,
  commanders,
  value,
  onChange,
}: {
  label: string;
  commanders: CommanderOption[];
  value: number | undefined;
  onChange: (id: number | undefined) => void;
}) {
  const selected = commanders.find((commander) => commander.id === value) ?? null;

  return (
    <Field>
      <Label>{label}</Label>
      <Combobox<CommanderOption>
        options={commanders}
        displayValue={(commander) => commander?.name ?? ""}
        placeholder="Type to search, or leave empty for any"
        aria-label={label}
        value={selected}
        onChange={(commander) => onChange(commander?.id)}
      >
        {(commander) => (
          <ComboboxOption value={commander}>
            <ComboboxLabel>{commander.name}</ComboboxLabel>
          </ComboboxOption>
        )}
      </Combobox>
      {selected ? (
        <Button plain className="mt-1 text-xs" onClick={() => onChange(undefined)}>
          Clear
        </Button>
      ) : null}
    </Field>
  );
}
