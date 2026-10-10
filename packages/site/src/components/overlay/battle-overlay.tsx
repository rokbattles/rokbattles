"use client";

import { type JSX, useEffect, useState } from "react";
import type { OverlayCommander } from "@/lib/overlay-commanders";
import { type BattleCardData, BattleCards } from "./battle-cards";

type Battle = BattleCardData & { expiresAt: number };
type Feed = { items: Battle[]; serverTime: number };

type BattleOverlayProps = {
  token: string;
  commanders: Record<string, OverlayCommander>;
};

const POLL_INTERVAL_MS = 3000;

export function BattleOverlay({ token, commanders }: BattleOverlayProps): JSX.Element {
  const [feed, setFeed] = useState<Feed>({ items: [], serverTime: 0 });
  const [now, setNow] = useState(0);
  const [status, setStatus] = useState("connecting");

  useEffect(() => {
    const controller = new AbortController();
    let pollTimer: ReturnType<typeof setTimeout>;
    let clock = { server: 0, received: performance.now() };

    const expiryTimer = setInterval(() => {
      setNow(clock.server + performance.now() - clock.received);
    }, 1000);

    async function poll(): Promise<void> {
      try {
        const response = await fetch(`/proxy/v1/overlay/${token}`, {
          cache: "no-store",
          credentials: "omit",
          referrerPolicy: "no-referrer",
          signal: AbortSignal.any([controller.signal, AbortSignal.timeout(10_000)]),
        });

        if (controller.signal.aborted) {
          return;
        }

        if (response.status === 404 || response.status === 401) {
          setFeed({ items: [], serverTime: 0 });
          setStatus("unavailable");

          return;
        }

        if (!response.ok) {
          throw new Error("Overlay feed unavailable");
        }

        const next: Feed = await response.json();

        if (controller.signal.aborted) {
          return;
        }

        clock = { server: next.serverTime, received: performance.now() };
        setNow(next.serverTime);
        setFeed(next);
        setStatus("live");
      } catch {
        if (controller.signal.aborted) {
          return;
        }

        setStatus("reconnecting");
      }

      pollTimer = setTimeout(poll, POLL_INTERVAL_MS);
    }

    void poll();

    return () => {
      controller.abort();
      clearTimeout(pollTimer);
      clearInterval(expiryTimer);
    };
  }, [token]);

  const visibleBattles = feed.items.filter((battle) => battle.expiresAt > now).slice(0, 7);

  return <BattleCards battles={visibleBattles} commanders={commanders} status={status} />;
}
