"use client";

import type { ReactNode } from "react";
import { createContext, useCallback, useState } from "react";
import type { ClaimedGovernor } from "@/lib/types/current-user";

export type GovernorContextValue = {
  activeGovernor?: ClaimedGovernor;
  governors: ClaimedGovernor[];
  setGovernors: (governors: ClaimedGovernor[]) => void;
  selectGovernor: (governorId: number) => void;
};

export const GovernorContext = createContext<GovernorContextValue | undefined>(undefined);

type GovernorProviderProps = {
  children: ReactNode;
  initialGovernors?: ClaimedGovernor[];
  initialActiveGovernorId?: number;
};

export function GovernorProvider({
  children,
  initialGovernors = [],
  initialActiveGovernorId,
}: GovernorProviderProps) {
  const [governors, setGovernorsState] = useState<ClaimedGovernor[]>(() => initialGovernors);
  const [activeGovernorId, setActiveGovernorId] = useState<number | undefined>(() => {
    if (
      initialActiveGovernorId != null &&
      initialGovernors.some((governor) => governor.governorId === initialActiveGovernorId)
    ) {
      return initialActiveGovernorId;
    }

    return initialGovernors[0]?.governorId;
  });

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

  const activeGovernor =
    activeGovernorId == null
      ? undefined
      : governors.find((governor) => governor.governorId === activeGovernorId);

  const value = {
    activeGovernor,
    governors,
    setGovernors,
    selectGovernor,
  };

  return <GovernorContext.Provider value={value}>{children}</GovernorContext.Provider>;
}
