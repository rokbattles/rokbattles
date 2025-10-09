"use client";

import { useCallback, useEffect, useRef, useState } from "react";

export interface CurrentUser {
  username: string;
  discriminator: string;
  globalName: string | null;
  email: string;
  avatar: string | null;
}

interface CurrentUserResponse {
  user: CurrentUser | null;
}

export function useCurrentUser() {
  const [user, setUser] = useState<CurrentUser | null>(null);
  const [loading, setLoading] = useState(true);
  const mountedRef = useRef(true);

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
        return;
      }

      if (!response.ok) {
        throw new Error("Failed to fetch current user");
      }

      setUser(payload?.user ?? null);
    } catch (err) {
      if (!mountedRef.current) {
        return;
      }

      console.error("Failed to fetch current user", err);
      setUser(null);
    } finally {
      if (mountedRef.current) {
        setLoading(false);
      }
    }
  }, []);

  useEffect(() => {
    mountedRef.current = true;
    void fetchUser();

    return () => {
      mountedRef.current = false;
    };
  }, [fetchUser]);

  return { user, loading, refresh: fetchUser };
}
