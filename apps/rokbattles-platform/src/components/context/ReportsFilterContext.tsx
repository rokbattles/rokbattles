"use client";

import type { Dispatch, ReactNode, SetStateAction } from "react";
import { createContext, useMemo, useState } from "react";

export type ReportsFilterType = "kvk" | "ark";

export type ReportsFilterContextValue = {
  playerId?: number;
  setPlayerId: Dispatch<SetStateAction<number | undefined>>;
  type?: ReportsFilterType;
  setType: Dispatch<SetStateAction<ReportsFilterType | undefined>>;
  primaryCommanderId?: number;
  setPrimaryCommanderId: Dispatch<SetStateAction<number | undefined>>;
  secondaryCommanderId?: number;
  setSecondaryCommanderId: Dispatch<SetStateAction<number | undefined>>;
  reset: () => void;
};

export const ReportsFilterContext = createContext<ReportsFilterContextValue | undefined>(undefined);

export function ReportsFilterProvider({ children }: { children: ReactNode }) {
  const [playerId, setPlayerId] = useState<number | undefined>();
  const [type, setType] = useState<ReportsFilterType | undefined>();
  const [primaryCommanderId, setPrimaryCommanderId] = useState<number | undefined>();
  const [secondaryCommanderId, setSecondaryCommanderId] = useState<number | undefined>();

  const reset = () => {
    setPlayerId(undefined);
    setType(undefined);
    setPrimaryCommanderId(undefined);
    setSecondaryCommanderId(undefined);
  };

  // biome-ignore lint/correctness/useExhaustiveDependencies: its safe
  const value = useMemo(
    () => ({
      playerId,
      setPlayerId,
      type,
      setType,
      primaryCommanderId,
      setPrimaryCommanderId,
      secondaryCommanderId,
      setSecondaryCommanderId,
      reset,
    }),
    [playerId, type, primaryCommanderId, secondaryCommanderId]
  );

  return <ReportsFilterContext.Provider value={value}>{children}</ReportsFilterContext.Provider>;
}
