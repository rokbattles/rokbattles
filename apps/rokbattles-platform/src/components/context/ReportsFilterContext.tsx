"use client";

import type { Dispatch, ReactNode, SetStateAction } from "react";
import { createContext, useMemo, useState } from "react";

export type ReportsFilterType = "kvk" | "ark";

export type ReportsFilterContextValue = {
  playerId?: number;
  setPlayerId: Dispatch<SetStateAction<number | undefined>>;
  type?: ReportsFilterType;
  setType: Dispatch<SetStateAction<ReportsFilterType | undefined>>;
};

export const ReportsFilterContext = createContext<ReportsFilterContextValue | undefined>(undefined);

export function ReportsFilterProvider({ children }: { children: ReactNode }) {
  const [playerId, setPlayerId] = useState<number | undefined>();
  const [type, setType] = useState<ReportsFilterType | undefined>();

  const value = useMemo(() => ({ playerId, setPlayerId, type, setType }), [playerId, type]);

  return <ReportsFilterContext.Provider value={value}>{children}</ReportsFilterContext.Provider>;
}
