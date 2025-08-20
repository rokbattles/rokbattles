import { Forward, Funnel, Star, Swords } from "lucide-react";

export default function LivePage() {
  return (
    <div className="relative isolate flex min-h-svh w-full max-lg:flex-col bg-zinc-900 lg:bg-zinc-950">
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
            <div className="flex items-center gap-1.5 text-sm text-zinc-300">
              <Funnel className="size-3.5 text-zinc-400" aria-hidden="true" />
              <span className="select-none">Filter</span>
            </div>
          </div>
          <div className="border-t border-white/10" />
          <div className="min-h-0 flex-1 overflow-y-auto">
            <div className="flex items-center justify-between py-3 px-5 hover:bg-zinc-800 cursor-pointer transition">
              <div className="w-36">
                <div className="truncate text-sm font-medium text-zinc-100">Gorgo</div>
                <div className="truncate text-xs text-zinc-400">Heraclius</div>
              </div>
              <div className="mx-2 flex shrink-0 items-center justify-center">
                <Swords className="size-5 text-zinc-300" aria-hidden="true" />
              </div>
              <div className="w-36 text-right">
                <div className="truncate text-sm font-medium text-zinc-100">Ashurbanipal</div>
                <div className="truncate text-xs text-zinc-400">Zhuge Liang</div>
              </div>
            </div>
            {Array.from({ length: 20 }).map((_, i) => (
              <div key={i} className="flex items-center justify-between py-3 px-5">
                <div className="w-36">
                  <div className="h-3.5 w-24 animate-pulse rounded bg-zinc-800/80" />
                  <div className="mt-1 h-2.5 w-16 animate-pulse rounded bg-zinc-800/70" />
                </div>
                <div className="mx-2 size-5 rounded bg-zinc-800/80" />
                <div className="w-36 text-right">
                  <div className="ml-auto h-3.5 w-28 animate-pulse rounded bg-zinc-800/80" />
                  <div className="ml-auto mt-1 h-2.5 w-20 animate-pulse rounded bg-zinc-800/70" />
                </div>
              </div>
            ))}
          </div>
        </div>
      </aside>
      <main className="flex flex-1 flex-col lg:min-w-0 lg:pl-96">
        <div className="grow p-6 lg:bg-zinc-900 lg:shadow-xs lg:ring-1 lg:ring-white/10">
          <div className="max-w-6xl mx-auto">
            <div className="flex items-center justify-between gap-3">
              <h1 className="text-xl font-semibold text-zinc-100">Battle Report</h1>
              <div className="flex items-center gap-2">
                <button
                  type="button"
                  className="group inline-flex items-center gap-1.5 rounded-md border border-white/10 bg-zinc-900/60 px-2.5 py-1.5 text-sm text-zinc-200 hover:bg-zinc-800/60 focus:outline-none focus:ring-2 focus:ring-white/20"
                >
                  <Star
                    className="size-4 text-zinc-300 group-hover:text-amber-400"
                    aria-hidden="true"
                  />
                  <span className="tabular-nums text-zinc-300">128</span>
                </button>
                <button
                  type="button"
                  className="inline-flex items-center gap-1.5 rounded-md border border-white/10 bg-zinc-900/60 px-2.5 py-1.5 text-sm text-zinc-200 hover:bg-zinc-800/60 focus:outline-none focus:ring-2 focus:ring-white/20"
                >
                  <Forward className="size-4 text-zinc-300" aria-hidden="true" />
                  <span className="hidden sm:inline">Share</span>
                </button>
              </div>
            </div>
            <div className="my-4 border-t border-white/10" />
          </div>
        </div>
      </main>
    </div>
  );
}
