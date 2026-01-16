"use client";

import { parseAsInteger, parseAsString, useQueryState } from "nuqs";
import type { Dispatch, ReactNode, SetStateAction } from "react";
import { createContext } from "react";

export type ReportsFilterType = "kvk" | "ark" | "home";
export type ReportsFilterSide = "none" | "sender" | "opponent" | "both";
export type ReportsGarrisonBuildingType = "flag" | "fortress" | "other";

const filterTypes = new Set<ReportsFilterType>(["kvk", "ark", "home"]);
const filterSides = new Set<ReportsFilterSide>([
  "none",
  "sender",
  "opponent",
  "both",
]);
const garrisonBuildingTypes = new Set<ReportsGarrisonBuildingType>([
  "flag",
  "fortress",
  "other",
]);

function resolveFilterType(
  value: string | null
): ReportsFilterType | undefined {
  if (!value) {
    return undefined;
  }
  return filterTypes.has(value as ReportsFilterType)
    ? (value as ReportsFilterType)
    : undefined;
}

function resolveSide(value: string | null): ReportsFilterSide {
  if (!value) {
    return "none";
  }
  return filterSides.has(value as ReportsFilterSide)
    ? (value as ReportsFilterSide)
    : "none";
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

export interface ReportsFilterContextValue {
  playerId?: number;
  setPlayerId: Dispatch<SetStateAction<number | undefined>>;
  type?: ReportsFilterType;
  setType: Dispatch<SetStateAction<ReportsFilterType | undefined>>;
  senderPrimaryCommanderId?: number;
  setSenderPrimaryCommanderId: Dispatch<SetStateAction<number | undefined>>;
  senderSecondaryCommanderId?: number;
  setSenderSecondaryCommanderId: Dispatch<SetStateAction<number | undefined>>;
  opponentPrimaryCommanderId?: number;
  setOpponentPrimaryCommanderId: Dispatch<SetStateAction<number | undefined>>;
  opponentSecondaryCommanderId?: number;
  setOpponentSecondaryCommanderId: Dispatch<SetStateAction<number | undefined>>;
  rallySide: ReportsFilterSide;
  setRallySide: Dispatch<SetStateAction<ReportsFilterSide>>;
  garrisonSide: ReportsFilterSide;
  setGarrisonSide: Dispatch<SetStateAction<ReportsFilterSide>>;
  garrisonBuildingType?: ReportsGarrisonBuildingType;
  setGarrisonBuildingType: Dispatch<
    SetStateAction<ReportsGarrisonBuildingType | undefined>
  >;
  reset: () => void;
}

export const ReportsFilterContext = createContext<
  ReportsFilterContextValue | undefined
>(undefined);

export function ReportsFilterProvider({ children }: { children: ReactNode }) {
  const [playerIdParam, setPlayerIdParam] = useQueryState(
    "pid",
    parseAsInteger
  );
  const [typeParam, setTypeParam] = useQueryState("type", parseAsString);
  const [senderPrimaryCommanderParam, setSenderPrimaryCommanderParam] =
    useQueryState("spc", parseAsInteger);
  const [senderSecondaryCommanderParam, setSenderSecondaryCommanderParam] =
    useQueryState("ssc", parseAsInteger);
  const [opponentPrimaryCommanderParam, setOpponentPrimaryCommanderParam] =
    useQueryState("opc", parseAsInteger);
  const [opponentSecondaryCommanderParam, setOpponentSecondaryCommanderParam] =
    useQueryState("osc", parseAsInteger);
  const [rallySideParam, setRallySideParam] = useQueryState(
    "rs",
    parseAsString
  );
  const [garrisonSideParam, setGarrisonSideParam] = useQueryState(
    "gs",
    parseAsString
  );
  const [garrisonBuildingParam, setGarrisonBuildingParam] = useQueryState(
    "gb",
    parseAsString
  );

  const playerId = playerIdParam ?? undefined;
  const type = resolveFilterType(typeParam);
  const senderPrimaryCommanderId = senderPrimaryCommanderParam ?? undefined;
  const senderSecondaryCommanderId = senderSecondaryCommanderParam ?? undefined;
  const opponentPrimaryCommanderId = opponentPrimaryCommanderParam ?? undefined;
  const opponentSecondaryCommanderId =
    opponentSecondaryCommanderParam ?? undefined;
  const rallySide = resolveSide(rallySideParam);
  const garrisonSide = resolveSide(garrisonSideParam);
  const garrisonBuildingType = resolveGarrisonBuildingType(
    garrisonBuildingParam
  );

  const setPlayerId: Dispatch<SetStateAction<number | undefined>> = (value) => {
    const next = typeof value === "function" ? value(playerId) : value;
    setPlayerIdParam(next ?? null);
  };

  const setType: Dispatch<SetStateAction<ReportsFilterType | undefined>> = (
    value
  ) => {
    const next = typeof value === "function" ? value(type) : value;
    setTypeParam(next ?? null);
  };

  const setSenderPrimaryCommanderId: Dispatch<
    SetStateAction<number | undefined>
  > = (value) => {
    const next =
      typeof value === "function" ? value(senderPrimaryCommanderId) : value;
    setSenderPrimaryCommanderParam(next ?? null);
  };

  const setSenderSecondaryCommanderId: Dispatch<
    SetStateAction<number | undefined>
  > = (value) => {
    const next =
      typeof value === "function" ? value(senderSecondaryCommanderId) : value;
    setSenderSecondaryCommanderParam(next ?? null);
  };

  const setOpponentPrimaryCommanderId: Dispatch<
    SetStateAction<number | undefined>
  > = (value) => {
    const next =
      typeof value === "function" ? value(opponentPrimaryCommanderId) : value;
    setOpponentPrimaryCommanderParam(next ?? null);
  };

  const setOpponentSecondaryCommanderId: Dispatch<
    SetStateAction<number | undefined>
  > = (value) => {
    const next =
      typeof value === "function" ? value(opponentSecondaryCommanderId) : value;
    setOpponentSecondaryCommanderParam(next ?? null);
  };

  const setRallySide: Dispatch<SetStateAction<ReportsFilterSide>> = (value) => {
    const next = typeof value === "function" ? value(rallySide) : value;
    setRallySideParam(next === "none" ? null : next);
  };

  const setGarrisonSide: Dispatch<SetStateAction<ReportsFilterSide>> = (
    value
  ) => {
    const next = typeof value === "function" ? value(garrisonSide) : value;
    setGarrisonSideParam(next === "none" ? null : next);
  };

  const setGarrisonBuildingType: Dispatch<
    SetStateAction<ReportsGarrisonBuildingType | undefined>
  > = (value) => {
    const next =
      typeof value === "function" ? value(garrisonBuildingType) : value;
    setGarrisonBuildingParam(next ?? null);
  };

  const reset = () => {
    setPlayerId(undefined);
    setType(undefined);
    setSenderPrimaryCommanderId(undefined);
    setSenderSecondaryCommanderId(undefined);
    setOpponentPrimaryCommanderId(undefined);
    setOpponentSecondaryCommanderId(undefined);
    setRallySide("none");
    setGarrisonSide("none");
    setGarrisonBuildingType(undefined);
  };

  const value = {
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
  };

  return (
    <ReportsFilterContext.Provider value={value}>
      {children}
    </ReportsFilterContext.Provider>
  );
}
