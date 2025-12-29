"use client";

import { useCallback, useContext, useEffect, useRef, useState } from "react";
import { GovernorContext } from "@/components/context/GovernorContext";
import type { CurrentUser } from "@/lib/types/current-user";

interface CurrentUserResponse {
  user: CurrentUser | null;
}

type UseCurrentUserOptions = {
  initialUser?: CurrentUser | null;
};

type FetchUserOptions = {
  showLoading?: boolean;
};

export function useCurrentUser(options: UseCurrentUserOptions = {}) {
  const { initialUser } = options;
  const hasInitialUser = initialUser !== undefined;
  const [user, setUser] = useState<CurrentUser | null>(initialUser ?? null);
  const [loading, setLoading] = useState(!hasInitialUser);
  const mountedRef = useRef(true);
  const { setGovernors } = useContext(GovernorContext);

  const fetchUser = useCallback(
    async ({ showLoading = true }: FetchUserOptions = {}) => {
      if (!mountedRef.current) {
        return;
      }

      if (showLoading) {
        setLoading(true);
      }

      try {
        const response = await fetch("/api/auth/me");
        const payload = (await response.json()) as CurrentUserResponse;

        if (!mountedRef.current) {
          return;
        }

        if (response.status === 401) {
          setUser(null);
          setGovernors([]);
          return;
        }

        if (!response.ok) {
          throw new Error("Failed to fetch current user");
        }

        const nextUser = payload?.user ?? null;
        setUser(nextUser);
        setGovernors(nextUser?.claimedGovernors ?? []);
      } catch (err) {
        if (!mountedRef.current) {
          return;
        }

        console.error("Failed to fetch current user", err);
        setUser(null);
        setGovernors([]);
      } finally {
        if (mountedRef.current && showLoading) {
          setLoading(false);
        }
      }
    },
    [setGovernors]
  );

  useEffect(() => {
    mountedRef.current = true;
    void fetchUser({ showLoading: !hasInitialUser });

    return () => {
      mountedRef.current = false;
    };
  }, [fetchUser, hasInitialUser]);

  const refresh = useCallback(() => fetchUser({ showLoading: true }), [fetchUser]);

  return {
    user,
    loading,
    refresh,
  };
}
