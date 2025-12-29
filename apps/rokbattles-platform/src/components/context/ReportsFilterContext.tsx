"use client";

import type { Dispatch, ReactNode, SetStateAction } from "react";
import { createContext, useState } from "react";

export type ReportsFilterType = "kvk" | "ark";

export type ReportsFilterContextValue = {
  playerId?: number;
  setPlayerId: Dispatch<SetStateAction<number | undefined>>;
  type?: ReportsFilterType;
  setType: Dispatch<SetStateAction<ReportsFilterType | undefined>>;
  rallyOnly: boolean;
  setRallyOnly: Dispatch<SetStateAction<boolean>>;
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
  const [rallyOnly, setRallyOnly] = useState(false);
  const [primaryCommanderId, setPrimaryCommanderId] = useState<number | undefined>();
  const [secondaryCommanderId, setSecondaryCommanderId] = useState<number | undefined>();

  const reset = () => {
    setPlayerId(undefined);
    setType(undefined);
    setRallyOnly(false);
    setPrimaryCommanderId(undefined);
    setSecondaryCommanderId(undefined);
  };

  const value = {
    playerId,
    setPlayerId,
    type,
    setType,
    rallyOnly,
    setRallyOnly,
    primaryCommanderId,
    setPrimaryCommanderId,
    secondaryCommanderId,
    setSecondaryCommanderId,
    reset,
  };

  return <ReportsFilterContext.Provider value={value}>{children}</ReportsFilterContext.Provider>;
}
