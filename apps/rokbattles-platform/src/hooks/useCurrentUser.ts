"use client";

import { useCallback, useContext, useEffect, useRef, useState } from "react";
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
      if (mountedRef.current) {
        setLoading(false);
      }
    }
  }, [setGovernors]);

  useEffect(() => {
    mountedRef.current = true;
    void fetchUser();

    return () => {
      mountedRef.current = false;
    };
  }, [fetchUser]);

  return {
    user,
    loading,
    refresh: fetchUser,
  };
}
