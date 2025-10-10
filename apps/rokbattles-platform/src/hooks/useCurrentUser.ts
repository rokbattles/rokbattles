"use client";

import { useCallback, useContext, useEffect, useMemo, useRef, useState } from "react";
import { GovernorContext } from "@/components/context/GovernorContext";

export interface ClaimedGovernor {
  governorId: number;
  governorName: string | null;
  governorAvatar: string | null;
}

export interface CurrentUser {
  username: string;
  discriminator: string;
  globalName: string | null;
  email: string;
  avatar: string | null;
  claimedGovernors: ClaimedGovernor[];
}

interface CurrentUserResponse {
  user: CurrentUser | null;
}

export function useCurrentUser() {
  const [user, setUser] = useState<CurrentUser | null>(null);
  const [loading, setLoading] = useState(true);
  const mountedRef = useRef(true);
  const { setGovernors } = useContext(GovernorContext);

  const syncGovernors = useCallback(
    (nextGovernors: ClaimedGovernor[]) => {
      setGovernors(nextGovernors);
    },
    [setGovernors]
  );

  const fetchUser = useCallback(async () => {
    if (!mountedRef.current) {
      return;
    }

    setLoading(true);

    try {
      const response = await fetch("/api/auth/me");
      const payload = (await response.json()) as CurrentUserResponse;

      if (!mountedRef.current) {
        return;
      }

      if (response.status === 401) {
        setUser(null);
        syncGovernors([]);
        return;
      }

      if (!response.ok) {
        throw new Error("Failed to fetch current user");
      }

      const nextUser = payload?.user ?? null;
      setUser(nextUser);
      syncGovernors(nextUser?.claimedGovernors ?? []);
    } catch (err) {
      if (!mountedRef.current) {
        return;
      }

      console.error("Failed to fetch current user", err);
      setUser(null);
      syncGovernors([]);
    } finally {
      if (mountedRef.current) {
        setLoading(false);
      }
    }
  }, [syncGovernors]);

  useEffect(() => {
    mountedRef.current = true;
    void fetchUser();

    return () => {
      mountedRef.current = false;
    };
  }, [fetchUser]);

  const value = useMemo(
    () => ({
      user,
      loading,
      refresh: fetchUser,
    }),
    [fetchUser, loading, user]
  );

  return value;
}
