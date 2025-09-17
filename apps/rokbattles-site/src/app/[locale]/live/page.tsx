import type { Locale } from "next-intl";
import { hasLocale } from "next-intl";
import { setRequestLocale } from "next-intl/server";
import { resolveNames } from "@/actions/datasets";
import { fetchSingleReport } from "@/actions/live-reports";
import { BattleReport } from "@/components/BattleReport";
import LiveSidebar from "@/components/LiveSidebar";
import { routing } from "@/i18n/routing";
import type { ReportsResponse, SingleReportItem } from "@/lib/types/reports";
import { formatUTCShort } from "@/lib/utc";

function getFirstValue(value: string | string[] | undefined) {
  if (Array.isArray(value)) {
    return value[0];
  }
  return value;
}

export default async function Page({ params, searchParams }: PageProps<"/[locale]/live">) {
  const [routeParams, queryParams] = await Promise.all([params, searchParams]);
  const normalizedLocale = hasLocale(routing.locales, routeParams.locale)
    ? routeParams.locale
    : routing.defaultLocale;
  const locale: Locale = normalizedLocale;

  setRequestLocale(locale);

  const hash = getFirstValue(queryParams.hash);
  const playerIdParam = getFirstValue(queryParams.player_id);
  const kvkParam = getFirstValue(queryParams.kvk_only);
  const arkParam = getFirstValue(queryParams.ark_only);

  const apiBase = process.env.ROKB_API_URL ?? "http://localhost:4445";
  const query = new URLSearchParams();

  if (playerIdParam && /^\d+$/.test(playerIdParam)) {
    query.set("player_id", playerIdParam);
  }
  if (kvkParam === "true") query.set("kvk_only", "true");
  if (arkParam === "true") query.set("ark_only", "true");

  let data: ReportsResponse | null = null;
  try {
    const url =
      query.size > 0 ? `${apiBase}/v1/reports?${query.toString()}` : `${apiBase}/v1/reports`;
    const res = await fetch(url, { cache: "no-store" });
    if (res.ok) {
      data = (await res.json()) as ReportsResponse;
    }
  } catch {
    data = null;
  }

  const items = data?.items ?? [];

  const entries = items.flatMap((item) =>
    item.entries.map((entry) => ({
      key: item.hash,
      ...entry,
    }))
  );

  const commanderIds = Array.from(
    new Set(
      entries
        .flatMap((entry) => [
          entry.self_commander_id,
          entry.self_secondary_commander_id,
          entry.enemy_commander_id,
          entry.enemy_secondary_commander_id,
        ])
        .filter((value): value is number => typeof value === "number" && Number.isFinite(value))
        .map((value) => String(value))
    )
  );

  const nameMap =
    commanderIds.length > 0 ? await resolveNames("commanders", commanderIds, locale) : {};

  let selectedItems: SingleReportItem[] = [];
  if (hash && hash.length > 0) {
    const report = await fetchSingleReport(hash);
    selectedItems = (report.items ?? []).slice();
  }

  return (
    <div className="relative isolate flex min-h-svh w-full max-lg:flex-col bg-zinc-900 lg:bg-zinc-950">
      <LiveSidebar
        initialItems={items}
        initialNameMap={nameMap}
        initialNextCursor={data?.next_cursor}
        locale={locale}
      />
      <main className="flex flex-1 flex-col lg:min-w-0 lg:pl-96">
        <div className="grow p-6 lg:bg-zinc-900 lg:shadow-xs lg:ring-1 lg:ring-white/10">
          {selectedItems.length > 0 ? (
            <div className="max-w-6xl mx-auto">
              <h1 className="text-xl font-semibold text-zinc-100 mb-4">Report Details</h1>
              {selectedItems.map((item) => {
                const startTimestamp = item.report?.metadata?.start_date;
                const endTimestamp = item.report?.metadata?.end_date;
                const label = formatUTCShort(startTimestamp, endTimestamp) ?? "Unknown";
                return (
                  <div key={item.hash}>
                    <div className="my-6 flex items-center gap-3">
                      <div className="h-px flex-1 bg-white/10" />
                      <div className="text-xs text-zinc-400">{label}</div>
                      <div className="h-px flex-1 bg-white/10" />
                    </div>
                    <BattleReport item={item} locale={locale} />
                  </div>
                );
              })}
            </div>
          ) : (
            <div className="max-w-6xl mx-auto">
              <h1 className="text-xl font-semibold text-zinc-100">Live Reports</h1>
              <p className="mt-2 text-sm text-zinc-400">Select a report from the left sidebar.</p>
            </div>
          )}
        </div>
      </main>
    </div>
  );
}
