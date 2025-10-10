"use client";

import type { ReactNode } from "react";
import { createContext, useCallback, useMemo, useState } from "react";
import type { ClaimedGovernor } from "@/hooks/useCurrentUser";

export type GovernorContextValue = {
  activeGovernor?: ClaimedGovernor;
  governors: ClaimedGovernor[];
  setGovernors: (governors: ClaimedGovernor[]) => void;
  selectGovernor: (governorId: number) => void;
};

export const GovernorContext = createContext<GovernorContextValue | undefined>(undefined);

export function GovernorProvider({ children }: { children: ReactNode }) {
  const [governors, setGovernorsState] = useState<ClaimedGovernor[]>([]);
  const [activeGovernorId, setActiveGovernorId] = useState<number | undefined>();

  const setGovernors = useCallback((nextGovernors: ClaimedGovernor[]) => {
    setGovernorsState(nextGovernors);
    setActiveGovernorId((currentId) => {
      if (nextGovernors.length === 0) {
        return undefined;
      }

      if (currentId != null && nextGovernors.some((gov) => gov.governorId === currentId)) {
        return currentId;
      }

      return nextGovernors[0]?.governorId;
    });
  }, []);

  const selectGovernor = useCallback(
    (governorId: number) => {
      setActiveGovernorId((currentId) => {
        if (currentId === governorId) {
          return currentId;
        }

        if (governors.some((governor) => governor.governorId === governorId)) {
          return governorId;
        }

        if (currentId != null && governors.some((governor) => governor.governorId === currentId)) {
          return currentId;
        }

        return governors[0]?.governorId;
      });
    },
    [governors]
  );

  const activeGovernor = useMemo(
    () =>
      activeGovernorId == null
        ? undefined
        : governors.find((governor) => governor.governorId === activeGovernorId),
    [activeGovernorId, governors]
  );

  const value = useMemo(
    () => ({
      activeGovernor,
      governors,
      setGovernors,
      selectGovernor,
    }),
    [activeGovernor, governors, selectGovernor, setGovernors]
  );

  return <GovernorContext.Provider value={value}>{children}</GovernorContext.Provider>;
}
