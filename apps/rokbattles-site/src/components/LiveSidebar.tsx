"use client";

import type { Route } from "next";
import { usePathname, useRouter, useSearchParams } from "next/navigation";
import { useEffect, useId, useMemo, useRef, useState } from "react";
import { fetchLiveReports } from "@/actions/live-reports";
import { LiveReportListItem } from "@/components/live/LiveReportListItem";
import type { ReportItem } from "@/lib/types/reports";

function flattenItems(items: ReportItem[]) {
  return items.flatMap((it) =>
    it.entries.map((e, idx) => ({ key: `${it.hash}:${idx}`, parent: it.hash, index: idx, ...e }))
  );
}

export default function LiveSidebar({
  initialItems,
  initialNameMap,
  initialNextCursor,
}: {
  initialItems: ReportItem[];
  initialNameMap: Record<string, string | undefined>;
  initialNextCursor?: string;
}) {
  const scrollRef = useRef<HTMLDivElement | null>(null);
  const sentinelRef = useRef<HTMLDivElement | null>(null);
  const inputId = useId();

  const [items, setItems] = useState<ReportItem[]>(() => initialItems ?? []);
  const [nameMap, setNameMap] = useState<Record<string, string | undefined>>(
    () => initialNameMap ?? {}
  );
  const [nextCursor, setNextCursor] = useState<string | undefined>(initialNextCursor);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [endReached, setEndReached] = useState<boolean>(!initialNextCursor);

  const flat = useMemo(() => flattenItems(items), [items]);
  const seenKeysRef = useRef<Set<string>>(new Set(flat.map((e) => e.key)));

  useEffect(() => {
    seenKeysRef.current = new Set(flat.map((e) => e.key));
  }, [flat]);

  const sp = useSearchParams();
  const pathname = usePathname();
  const router = useRouter();
  const toRoute = (params: URLSearchParams): Route => {
    const query = params.toString();
    const basePath = pathname ?? "/live";
    return (query ? `${basePath}?${query}` : basePath) as Route;
  };
  const locParam = sp?.get("locale");
  const currentLocale = locParam === "es" || locParam === "kr" ? (locParam as "es" | "kr") : "en";

  const pidParam = sp?.get("player_id");
  const kvkParam = sp?.get("kvk_only") === "true";
  const arkParam = sp?.get("ark_only") === "true";

  const [playerId, setPlayerId] = useState<string>(
    pidParam && /^\d+$/.test(pidParam) ? pidParam : ""
  );
  const initialMode: "all" | "kvk" | "ark" = kvkParam ? "kvk" : arkParam ? "ark" : "all";
  const [mode, setMode] = useState<"all" | "kvk" | "ark">(initialMode);

  useEffect(() => {
    setItems([]);
    setNextCursor(undefined);
    setEndReached(false);
    seenKeysRef.current = new Set();

    const pid = playerId && /^\d+$/.test(playerId) ? Number(playerId) : undefined;
    const kvkOnly = mode === "kvk";
    const arkOnly = mode === "ark";

    fetchLiveReports(undefined, currentLocale, { playerId: pid, kvkOnly, arkOnly })
      .then((data) => {
        setNameMap(data.names ?? {});
        setItems(data.items ?? []);
        setNextCursor(data.next_cursor);
        setEndReached(!data.next_cursor);
      })
      .catch(() => {});
  }, [currentLocale, mode, playerId]);

  useEffect(() => {
    if (!sentinelRef.current) return;

    const root = scrollRef.current ?? null;
    const io = new IntersectionObserver(
      (entries) => {
        const entry = entries[0];

        if (!entry.isIntersecting) return;
        if (loading || endReached || !nextCursor) return;

        setLoading(true);
        setError(null);

        const pid = playerId && /^\d+$/.test(playerId) ? Number(playerId) : undefined;
        const kvkOnly = mode === "kvk";
        const arkOnly = mode === "ark";

        fetchLiveReports(nextCursor, currentLocale, { playerId: pid, kvkOnly, arkOnly })
          .then((data) => {
            const newItems = data.items ?? [];
            const newNames = data.names ?? {};

            setNameMap((prev) => ({ ...prev, ...newNames }));

            if (newItems.length > 0) {
              const newFlat = flattenItems(newItems);
              const filtered = newFlat.filter((e) => !seenKeysRef.current.has(e.key));

              if (filtered.length > 0) {
                setItems((prev) => [...prev, ...newItems]);
                filtered.forEach((e) => {
                  seenKeysRef.current.add(e.key);
                });
              }
            }

            const next = data.next_cursor;
            setNextCursor(next);
            if (!next) setEndReached(true);
          })
          .catch((err) => {
            setError(err instanceof Error ? err.message : String(err));
          })
          .finally(() => setLoading(false));
      },
      { root, rootMargin: "0px", threshold: 1.0 }
    );

    io.observe(sentinelRef.current);
    return () => io.disconnect();
  }, [nextCursor, loading, endReached, currentLocale, mode, playerId]);

  // biome-ignore lint/correctness/useExhaustiveDependencies: ignore for now
  useEffect(() => {
    const params = new URLSearchParams(Array.from(sp?.entries?.() ?? []));

    if (playerId && /^\d+$/.test(playerId)) {
      params.set("player_id", playerId);
    } else {
      params.delete("player_id");
    }

    params.delete("kvk_only");
    params.delete("ark_only");

    if (mode === "kvk") params.set("kvk_only", "true");
    if (mode === "ark") params.set("ark_only", "true");

    router.replace(toRoute(params));
  }, [playerId, mode, pathname]);

  return (
    <aside className="fixed inset-y-0 left-0 w-96 max-lg:hidden">
      <div className="flex h-full flex-col">
        <div className="flex items-center justify-between p-5">
          <div className="flex items-center gap-2">
            <span className="relative flex size-2.5">
              <span className="absolute inline-flex h-full w-full rounded-full bg-red-400 opacity-75 animate-ping"></span>
              <span className="relative inline-flex rounded-full size-2.5 bg-red-500"></span>
            </span>
            <span className="text-zinc-300 text-sm">Live mode</span>
          </div>
          <div className="flex items-center gap-2 text-sm text-zinc-300">
            <label htmlFor="locale" className="sr-only">
              Locale
            </label>
            {/* biome-ignore lint/correctness/useUniqueElementIds: ignore */}
            <select
              id="locale"
              value={currentLocale}
              onChange={(e) => {
                const next = e.target.value;
                const params = new URLSearchParams(Array.from(sp?.entries?.() ?? []));
                params.set("locale", next);
                router.replace(toRoute(params));
              }}
              className="rounded-md bg-zinc-800/60 px-2 py-1 text-xs text-zinc-100 ring-1 ring-inset ring-white/10"
            >
              <option value="en">English</option>
              <option value="es">Español</option>
              <option value="kr">한국어</option>
            </select>
          </div>
        </div>
        <div className="py-3 px-5 border-t border-b border-white/10">
          <div className="flex items-center gap-2 mb-2">
            <div className="text-sm/6 text-gray-300 w-28">Governor</div>
            <input
              id={inputId}
              inputMode="numeric"
              pattern="[0-9]*"
              value={playerId}
              onChange={(e) => {
                const onlyDigits = e.target.value.replace(/[^0-9]/g, "");
                setPlayerId(onlyDigits);
              }}
              placeholder="71738515"
              className="block w-full rounded-md px-2 py-1 outline-1 -outline-offset-1 focus:outline-2 focus:-outline-offset-2 sm:text-sm/6 bg-white/5 text-white outline-white/10 placeholder:text-gray-500 focus:outline-blue-500"
            />
          </div>
          <div className="flex items-center gap-3">
            <div className="text-sm/6 text-gray-300 w-20">Mode</div>
            <label className="flex items-center gap-1 text-xs text-zinc-300">
              <input
                type="radio"
                name="mode"
                value="all"
                checked={mode === "all"}
                onChange={() => setMode("all")}
                className="relative size-4 appearance-none rounded-full border before:absolute before:inset-1 before:rounded-full before:bg-white not-checked:before:hidden focus-visible:outline-2 focus-visible:outline-offset-2 border-white/10 bg-white/5 checked:border-blue-500 checked:bg-blue-500 focus-visible:outline-blue-500 disabled:border-white/5 disabled:bg-white/10 disabled:before:bg-white/20"
              />
              <span className="ml-1 block text-sm/6 font-medium text-white">All</span>
            </label>
            <label className="flex items-center gap-1 text-xs text-zinc-300">
              <input
                type="radio"
                name="mode"
                value="kvk"
                checked={mode === "kvk"}
                onChange={() => setMode("kvk")}
                className="relative size-4 appearance-none rounded-full border before:absolute before:inset-1 before:rounded-full before:bg-white not-checked:before:hidden focus-visible:outline-2 focus-visible:outline-offset-2 border-white/10 bg-white/5 checked:border-blue-500 checked:bg-blue-500 focus-visible:outline-blue-500 disabled:border-white/5 disabled:bg-white/10 disabled:before:bg-white/20"
              />
              <span className="ml-1 block text-sm/6 font-medium text-white">KVK</span>
            </label>
            <label className="flex items-center gap-1 text-xs text-zinc-300">
              <input
                type="radio"
                name="mode"
                value="ark"
                checked={mode === "ark"}
                onChange={() => setMode("ark")}
                className="relative size-4 appearance-none rounded-full border before:absolute before:inset-1 before:rounded-full before:bg-white not-checked:before:hidden focus-visible:outline-2 focus-visible:outline-offset-2 border-white/10 bg-white/5 checked:border-blue-500 checked:bg-blue-500 focus-visible:outline-blue-500 disabled:border-white/5 disabled:bg-white/10 disabled:before:bg-white/20"
              />
              <span className="ml-1 block text-sm/6 font-medium text-white">Ark</span>
            </label>
          </div>
        </div>
        <div ref={scrollRef} className="min-h-0 flex-1 overflow-y-auto">
          {error && <div className="p-5 text-sm text-red-400">Failed to load: {error}</div>}
          {flat.length === 0 && !loading && !error && (
            <div className="p-5 text-sm text-zinc-400">No reports found.</div>
          )}
          {flat.map((entry) => {
            const left = entry.self_commander_id
              ? (nameMap?.[String(entry.self_commander_id)] ?? String(entry.self_commander_id))
              : "Unknown";
            const right = entry.enemy_commander_id
              ? (nameMap?.[String(entry.enemy_commander_id)] ?? String(entry.enemy_commander_id))
              : "Unknown";
            const leftSecondary = entry.self_secondary_commander_id
              ? (nameMap?.[String(entry.self_secondary_commander_id)] ??
                String(entry.self_secondary_commander_id))
              : "";
            const rightSecondary = entry.enemy_secondary_commander_id
              ? (nameMap?.[String(entry.enemy_secondary_commander_id)] ??
                String(entry.enemy_secondary_commander_id))
              : "";
            const params = new URLSearchParams();
            params.set("hash", entry.parent);
            params.set("locale", currentLocale);

            if (playerId && /^\d+$/.test(playerId)) params.set("player_id", playerId);
            if (mode === "kvk") params.set("kvk_only", "true");
            if (mode === "ark") params.set("ark_only", "true");

            const query = Object.fromEntries(params.entries()) as Record<string, string>;

            return (
              <LiveReportListItem
                key={entry.key}
                left={left}
                right={right}
                leftSecondary={leftSecondary}
                rightSecondary={rightSecondary}
                query={query}
              />
            );
          })}
          <div ref={sentinelRef} />
          {loading && <div className="p-5 text-xs text-zinc-400">Loading more...</div>}
          {endReached && flat.length > 0 && (
            <div className="p-5 text-[11px] text-zinc-500">End of list</div>
          )}
        </div>
      </div>
    </aside>
  );
}
