"use client";

import { use, useCallback, useEffect, useRef, useState } from "react";
import type { CurrentUser } from "@/lib/types/current-user";
import { GovernorContext } from "@/providers/governor-context";

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
  const { setGovernors } = use(GovernorContext);

  const fetchUser = useCallback(
    async ({ showLoading = true }: FetchUserOptions = {}) => {
      if (!mountedRef.current) {
        return;
      }

      if (showLoading) {
        setLoading(true);
      }

      try {
        const response = await fetch("/proxy/v1/auth/me");

        if (!mountedRef.current) {
          return;
        }

        if (response.status === 401) {
          setUser(null);
          setGovernors([]);
          return;
        }

        if (!response.ok) {
          throw new Error("Failed to fetch data");
        }

        const payload = (await response.json()) as CurrentUserResponse;
        const nextUser = payload?.user ?? null;
        setUser(nextUser);
        setGovernors(nextUser?.claimedGovernors ?? []);
      } catch (err) {
        if (!mountedRef.current) {
          return;
        }

        console.error("Failed to fetch data", err);
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
    if (!hasInitialUser) {
      void fetchUser({ showLoading: true });
    }

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
