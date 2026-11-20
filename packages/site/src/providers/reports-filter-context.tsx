"use client";

import { parseAsInteger, parseAsString, useQueryState } from "nuqs";
import type { Dispatch, ReactNode, SetStateAction } from "react";
import { createContext } from "react";

export type ReportsTroopType = "infantry" | "cavalry" | "archer" | "leadership" | "integration";
export type ReportsFilterSide = "none" | "sender" | "opponent" | "both";
export type ReportsGarrisonBuildingType = "flag" | "fortress" | "other";

const troopTypes = new Set<ReportsTroopType>([
  "infantry",
  "cavalry",
  "archer",
  "leadership",
  "integration",
]);
const filterSides = new Set<ReportsFilterSide>(["none", "sender", "opponent", "both"]);
const garrisonBuildingTypes = new Set<ReportsGarrisonBuildingType>(["flag", "fortress", "other"]);

function resolveTroopType(value: string | null): ReportsTroopType | undefined {
  if (!value) {
    return undefined;
  }
  return troopTypes.has(value as ReportsTroopType) ? (value as ReportsTroopType) : undefined;
}

function resolveSide(value: string | null): ReportsFilterSide {
  if (!value) {
    return "none";
  }
  return filterSides.has(value as ReportsFilterSide) ? (value as ReportsFilterSide) : "none";
}

function resolveGarrisonBuildingType(
  value: string | null
): ReportsGarrisonBuildingType | undefined {
  if (!value) {
    return undefined;
  }
  return garrisonBuildingTypes.has(value as ReportsGarrisonBuildingType)
    ? (value as ReportsGarrisonBuildingType)
    : undefined;
}

export type ReportsFilterContextValue = {
  playerId?: number;
  setPlayerId: Dispatch<SetStateAction<number | undefined>>;
  troopType?: ReportsTroopType;
  setTroopType: Dispatch<SetStateAction<ReportsTroopType | undefined>>;
  tradeMin?: number;
  setTradeMin: Dispatch<SetStateAction<number | undefined>>;
  tradeMax?: number;
  setTradeMax: Dispatch<SetStateAction<number | undefined>>;
  commanderPrimaryId?: number;
  setCommanderPrimaryId: Dispatch<SetStateAction<number | undefined>>;
  commanderSecondaryId?: number;
  setCommanderSecondaryId: Dispatch<SetStateAction<number | undefined>>;
  rallySide: ReportsFilterSide;
  setRallySide: Dispatch<SetStateAction<ReportsFilterSide>>;
  garrisonSide: ReportsFilterSide;
  setGarrisonSide: Dispatch<SetStateAction<ReportsFilterSide>>;
  garrisonBuildingType?: ReportsGarrisonBuildingType;
  setGarrisonBuildingType: Dispatch<SetStateAction<ReportsGarrisonBuildingType | undefined>>;
  reset: () => void;
};

export const ReportsFilterContext = createContext<ReportsFilterContextValue | undefined>(undefined);

export function ReportsFilterProvider({ children }: { children: ReactNode }) {
  const [playerIdParam, setPlayerIdParam] = useQueryState("pid", parseAsInteger);
  const [troopTypeParam, setTroopTypeParam] = useQueryState("tt", parseAsString);
  const [tradeMinParam, setTradeMinParam] = useQueryState("tmin", parseAsInteger);
  const [tradeMaxParam, setTradeMaxParam] = useQueryState("tmax", parseAsInteger);
  const [commanderPrimaryParam, setCommanderPrimaryParam] = useQueryState("cpc", parseAsInteger);
  const [commanderSecondaryParam, setCommanderSecondaryParam] = useQueryState(
    "csc",
    parseAsInteger
  );
  const [rallySideParam, setRallySideParam] = useQueryState("rs", parseAsString);
  const [garrisonSideParam, setGarrisonSideParam] = useQueryState("gs", parseAsString);
  const [garrisonBuildingParam, setGarrisonBuildingParam] = useQueryState("gb", parseAsString);

  const playerId = playerIdParam ?? undefined;
  const troopType = resolveTroopType(troopTypeParam);
  const tradeMin = tradeMinParam ?? undefined;
  const tradeMax = tradeMaxParam ?? undefined;
  const commanderPrimaryId = commanderPrimaryParam ?? undefined;
  const commanderSecondaryId = commanderSecondaryParam ?? undefined;
  const rallySide = resolveSide(rallySideParam);
  const garrisonSide = resolveSide(garrisonSideParam);
  const garrisonBuildingType = resolveGarrisonBuildingType(garrisonBuildingParam);

  const setPlayerId: Dispatch<SetStateAction<number | undefined>> = (value) => {
    const next = typeof value === "function" ? value(playerId) : value;
    setPlayerIdParam(next ?? null);
  };

  const setTroopType: Dispatch<SetStateAction<ReportsTroopType | undefined>> = (value) => {
    const next = typeof value === "function" ? value(troopType) : value;
    setTroopTypeParam(next ?? null);
  };

  const setTradeMin: Dispatch<SetStateAction<number | undefined>> = (value) => {
    const next = typeof value === "function" ? value(tradeMin) : value;
    setTradeMinParam(next ?? null);
  };

  const setTradeMax: Dispatch<SetStateAction<number | undefined>> = (value) => {
    const next = typeof value === "function" ? value(tradeMax) : value;
    setTradeMaxParam(next ?? null);
  };

  const setCommanderPrimaryId: Dispatch<SetStateAction<number | undefined>> = (value) => {
    const next = typeof value === "function" ? value(commanderPrimaryId) : value;
    setCommanderPrimaryParam(next ?? null);
  };

  const setCommanderSecondaryId: Dispatch<SetStateAction<number | undefined>> = (value) => {
    const next = typeof value === "function" ? value(commanderSecondaryId) : value;
    setCommanderSecondaryParam(next ?? null);
  };

  const setRallySide: Dispatch<SetStateAction<ReportsFilterSide>> = (value) => {
    const next = typeof value === "function" ? value(rallySide) : value;
    setRallySideParam(next === "none" ? null : next);
  };

  const setGarrisonSide: Dispatch<SetStateAction<ReportsFilterSide>> = (value) => {
    const next = typeof value === "function" ? value(garrisonSide) : value;
    setGarrisonSideParam(next === "none" ? null : next);
  };

  const setGarrisonBuildingType: Dispatch<
    SetStateAction<ReportsGarrisonBuildingType | undefined>
  > = (value) => {
    const next = typeof value === "function" ? value(garrisonBuildingType) : value;
    setGarrisonBuildingParam(next ?? null);
  };

  const reset = () => {
    setPlayerId(undefined);
    setTroopType(undefined);
    setTradeMin(undefined);
    setTradeMax(undefined);
    setCommanderPrimaryId(undefined);
    setCommanderSecondaryId(undefined);
    setRallySide("none");
    setGarrisonSide("none");
    setGarrisonBuildingType(undefined);
  };

  const value = {
    playerId,
    setPlayerId,
    troopType,
    setTroopType,
    tradeMin,
    setTradeMin,
    tradeMax,
    setTradeMax,
    commanderPrimaryId,
    setCommanderPrimaryId,
    commanderSecondaryId,
    setCommanderSecondaryId,
    rallySide,
    setRallySide,
    garrisonSide,
    setGarrisonSide,
    garrisonBuildingType,
    setGarrisonBuildingType,
    reset,
  };

  return <ReportsFilterContext.Provider value={value}>{children}</ReportsFilterContext.Provider>;
}
