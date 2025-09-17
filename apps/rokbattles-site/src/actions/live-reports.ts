"use server";

import type { Locale } from "next-intl";
import { routing } from "@/i18n/routing";
import type {
  ReportsResponse,
  ReportsWithNamesResponse,
  SingleReportResponse,
} from "@/lib/types/reports";
import { resolveNames } from "./datasets";

export async function fetchLiveReports(
  after?: string,
  locale: Locale = routing.defaultLocale,
  opts?: { playerId?: number; kvkOnly?: boolean; arkOnly?: boolean }
): Promise<ReportsWithNamesResponse> {
  const apiBase = process.env.ROKB_API_URL ?? "http://localhost:4445";
  const base = `${apiBase}/v1/reports`;
  const params = new URLSearchParams();

  if (after) params.set("after", after);
  if (opts?.playerId && Number.isFinite(opts.playerId)) {
    params.set("player_id", String(opts.playerId));
  }
  if (opts?.kvkOnly) params.set("kvk_only", "true");
  if (opts?.arkOnly) params.set("ark_only", "true");

  const query = params.toString();
  const url = query ? `${base}?${query}` : base;

  let data: ReportsResponse | null = null;
  try {
    const res = await fetch(url, { cache: "no-store" });
    if (!res.ok) {
      return {
        items: [],
        next_cursor: undefined,
        count: 0,
        names: {},
        error: `Upstream error: ${res.status}`,
      };
    }
    data = (await res.json()) as ReportsResponse;
  } catch (err) {
    const msg = err instanceof Error ? err.message : String(err);
    return {
      items: [],
      next_cursor: undefined,
      count: 0,
      names: {},
      error: `Failed to fetch upstream: ${msg}`,
    };
  }

  const items = data?.items ?? [];

  const commanderIds = Array.from(
    new Set(
      items
        .flatMap((it) => it.entries)
        .flatMap((e) => [
          e.self_commander_id,
          e.self_secondary_commander_id,
          e.enemy_commander_id,
          e.enemy_secondary_commander_id,
        ])
        .filter((v): v is number => typeof v === "number" && !Number.isNaN(v))
        .map((n) => String(n))
    )
  );

  let names: Record<string, string | undefined> = {};
  try {
    if (commanderIds.length > 0) {
      names = await resolveNames("commanders", commanderIds, locale);
    }
  } catch {
    names = {};
  }

  return { items, next_cursor: data?.next_cursor, count: data?.count, names };
}

export async function fetchSingleReport(parentHash: string): Promise<SingleReportResponse> {
  const apiBase = process.env.ROKB_API_URL ?? "http://localhost:4445";
  const url = `${apiBase}/v1/report/${encodeURIComponent(parentHash)}`;

  let data: SingleReportResponse | null = null;
  const empty: SingleReportResponse = {
    parent_hash: parentHash,
    items: [],
    next_cursor: undefined,
    count: 0,
  };
  try {
    const res = await fetch(url, { cache: "no-store" });
    if (!res.ok) return empty;
    data = (await res.json()) as SingleReportResponse;
  } catch {
    return empty;
  }
  const items = data?.items ?? [];
  return {
    parent_hash: data?.parent_hash ?? parentHash,
    items,
    count: items.length,
    next_cursor: undefined,
  };
}
