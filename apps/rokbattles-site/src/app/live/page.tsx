import { resolveNames } from "@/actions/datasets";
import { fetchSingleReport } from "@/actions/live-reports";
import { BattleReport } from "@/components/BattleReport";
import LiveSidebar from "@/components/LiveSidebar";
import type { ReportsResponse, SingleReportItem } from "@/lib/types/reports";
import { formatUTCShort } from "@/lib/utc";

export default async function Page({
  searchParams,
}: {
  searchParams: Promise<{ [key: string]: string | string[] | undefined }>;
}) {
  const { hash, locale, player_id, kvk_only, ark_only } = await searchParams;

  let data: ReportsResponse | null = null;
  try {
    const apiBase = process.env.ROKB_API_URL ?? "http://localhost:4445";
    const search = new URLSearchParams();
    const pidStr = Array.isArray(player_id) ? player_id[0] : player_id;
    const pid = pidStr && /^\d+$/.test(pidStr) ? pidStr : undefined;
    const kvk = Array.isArray(kvk_only) ? kvk_only[0] : kvk_only;
    const ark = Array.isArray(ark_only) ? ark_only[0] : ark_only;

    if (pid) search.set("player_id", pid);
    if (kvk === "true") search.set("kvk_only", "true");
    if (ark === "true") search.set("ark_only", "true");

    const qs = search.toString();
    const url = qs ? `${apiBase}/v1/reports?${qs}` : `${apiBase}/v1/reports`;

    const res = await fetch(url, {
      cache: "no-store",
    });

    if (res.ok) {
      data = (await res.json()) as ReportsResponse;
    }
  } catch {}

  const items = data?.items ?? [];

  const entries = items.flatMap((it) =>
    it.entries.map((e, idx) => ({ key: `${it.hash}:${idx}`, ...e }))
  );
  const commanderIds = Array.from(
    new Set(
      entries
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

  const candidateLocale = Array.isArray(locale) ? locale[0] : locale;
  const finalLocale: "en" | "es" | "kr" =
    candidateLocale === "es" || candidateLocale === "kr" ? candidateLocale : "en";

  const nameMap =
    commanderIds.length > 0 ? await resolveNames("commanders", commanderIds, finalLocale) : {};

  const parentHash = Array.isArray(hash) ? hash[0] : hash;

  let selectedItems: SingleReportItem[] = [];
  if (parentHash && parentHash.length > 0) {
    const report = await fetchSingleReport(parentHash);
    selectedItems = (report.items ?? []).slice();
  }

  return (
    <div className="relative isolate flex min-h-svh w-full max-lg:flex-col bg-zinc-900 lg:bg-zinc-950">
      <LiveSidebar
        initialItems={items}
        initialNameMap={nameMap}
        initialNextCursor={data?.next_cursor}
      />
      <main className="flex flex-1 flex-col lg:min-w-0 lg:pl-96">
        <div className="grow p-6 lg:bg-zinc-900 lg:shadow-xs lg:ring-1 lg:ring-white/10">
          {selectedItems.length > 0 ? (
            <div className="max-w-6xl mx-auto">
              <h1 className="text-xl font-semibold text-zinc-100 mb-4">Report Details</h1>
              {selectedItems.map((it, idx) => {
                const ts = it.start_date ?? it.report?.metadata?.start_date;
                const label = formatUTCShort(ts) ?? "UTC \u2014";
                return (
                  <div key={`${it.hash}:${idx}`}>
                    <div className="my-6 flex items-center gap-3">
                      <div className="h-px flex-1 bg-white/10" />
                      <div className="text-xs text-zinc-400">{label}</div>
                      <div className="h-px flex-1 bg-white/10" />
                    </div>
                    <BattleReport item={it} locale={finalLocale} />
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
