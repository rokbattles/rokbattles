import type { Locale } from "next-intl";
import { setRequestLocale } from "next-intl/server";
import { resolveNames } from "@/actions/datasets";
import { fetchSingleReport } from "@/actions/live-reports";
import { BattleReport } from "@/components/BattleReport";
import LiveSidebar from "@/components/LiveSidebar";
import type { ReportsResponse, SingleReportItem } from "@/lib/types/reports";
import { formatUTCShort } from "@/lib/utc";

// TODO Replace async by using "use" from React
export default async function LivePage({ params, searchParams }: PageProps<"/[locale]/live">) {
  const { locale } = await params;
  setRequestLocale(locale as Locale);

  const sp = await searchParams;
  const hashParam = sp?.hash;

  let data: ReportsResponse | null = null;
  try {
    const apiBase = process.env.ROKB_API_URL ?? "http://localhost:4445";
    const res = await fetch(`${apiBase}/v1/reports`, {
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

  const nameMap =
    // @ts-expect-error underlying function needs to be updated to use Locale type
    commanderIds.length > 0 ? await resolveNames("commanders", commanderIds, locale as string) : {};

  const parentHash = Array.isArray(hashParam) ? hashParam[0] : hashParam;

  let selectedItems: SingleReportItem[] = [];
  if (parentHash && parentHash.length > 0) {
    const report = await fetchSingleReport(parentHash);
    selectedItems = (report.items ?? []).slice();
  }

  return (
    <div className="relative isolate flex min-h-svh w-full max-lg:flex-col bg-zinc-900 lg:bg-zinc-950">
      {/* TODO: Update component to passed in locale (or fetch it via next-intl hook) */}
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
                    {/* @ts-expect-error underlying function needs to be updated to use Locale type */}
                    <BattleReport item={it} locale={locale as string} />
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
