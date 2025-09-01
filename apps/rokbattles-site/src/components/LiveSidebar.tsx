"use client";

import { Swords } from "lucide-react";
import Link from "next/link";
import { usePathname, useRouter, useSearchParams } from "next/navigation";
import { useEffect, useMemo, useRef, useState } from "react";
import { fetchLiveReports } from "@/actions/live-reports";
import type { ReportItem } from "@/lib/types/reports";

function flattenItems(items: ReportItem[]) {
  return items.flatMap((it) =>
    it.entries.map((e, idx) => ({ key: `${it.hash}:${idx}`, parent: it.hash, index: idx, ...e }))
  );
}

function ListItem({
  left,
  right,
  leftSecondary,
  rightSecondary,
  itemKey,
  href,
}: {
  left: string;
  right: string;
  leftSecondary?: string;
  rightSecondary?: string;
  itemKey: string;
  href: string;
}) {
  return (
    <Link
      href={href}
      key={itemKey}
      className="flex items-center justify-between py-3 px-5 hover:bg-zinc-800 transition"
    >
      <div className="w-36">
        <div className="truncate text-sm font-medium text-zinc-100">{left}</div>
        <div className="truncate text-[11px] text-zinc-400">{leftSecondary ?? ""}</div>
      </div>
      <div className="mx-2 flex shrink-0 items-center justify-center">
        <Swords className="size-5 text-zinc-300" aria-hidden="true" />
      </div>
      <div className="w-36 text-right">
        <div className="truncate text-sm font-medium text-zinc-100">{right}</div>
        <div className="truncate text-[11px] text-zinc-400">{rightSecondary ?? ""}</div>
      </div>
    </Link>
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
  const locParam = sp?.get("locale");
  const currentLocale = locParam === "es" || locParam === "kr" ? (locParam as "es" | "kr") : "en";

  useEffect(() => {
    fetchLiveReports(undefined, currentLocale)
      .then((data) => setNameMap(data.names ?? {}))
      .catch(() => {});
  }, [currentLocale]);

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
        fetchLiveReports(nextCursor, currentLocale)
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
  }, [nextCursor, loading, endReached, currentLocale]);

  useEffect(() => {
    fetchLiveReports(undefined, currentLocale)
      .then((data) => setNameMap(data.names ?? {}))
      .catch(() => {});
  }, [currentLocale]);

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
                router.replace(`${pathname}?${params.toString()}`);
              }}
              className="rounded-md bg-zinc-800/60 px-2 py-1 text-xs text-zinc-100 ring-1 ring-inset ring-white/10"
            >
              <option value="en">English</option>
              <option value="es">Español</option>
              <option value="kr">한국어</option>
            </select>
          </div>
        </div>
        <div className="border-t border-white/10" />
        <div ref={scrollRef} className="min-h-0 flex-1 overflow-y-auto">
          {error && <div className="p-5 text-sm text-red-400">Failed to load: {error}</div>}
          {flat.length === 0 && !loading && !error && (
            <div className="p-5 text-sm text-zinc-400">No reports found.</div>
          )}
          {flat.map((e) => {
            const left = e.self_commander_id
              ? (nameMap?.[String(e.self_commander_id)] ?? String(e.self_commander_id))
              : "Unknown";
            const right = e.enemy_commander_id
              ? (nameMap?.[String(e.enemy_commander_id)] ?? String(e.enemy_commander_id))
              : "Unknown";
            const leftSecondary = e.self_secondary_commander_id
              ? (nameMap?.[String(e.self_secondary_commander_id)] ??
                String(e.self_secondary_commander_id))
              : "";
            const rightSecondary = e.enemy_secondary_commander_id
              ? (nameMap?.[String(e.enemy_secondary_commander_id)] ??
                String(e.enemy_secondary_commander_id))
              : "";
            const href = `/live?hash=${encodeURIComponent(e.parent)}&locale=${encodeURIComponent(currentLocale)}`;
            return (
              <ListItem
                key={e.key}
                itemKey={e.key}
                left={left}
                right={right}
                leftSecondary={leftSecondary}
                rightSecondary={rightSecondary}
                href={href}
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
