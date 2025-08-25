import { Forward, Star } from "lucide-react";
import type React from "react";
import type { SVGProps } from "react";
import { cn } from "@/lib/cn";

export default function Page() {
  return (
    <div className="relative min-h-dvh bg-zinc-950 text-zinc-100 antialiased">
      <div aria-hidden="true" className="pointer-events-none absolute inset-0 mx-auto max-w-7xl">
        <div className="absolute left-0 top-0 h-full w-px bg-gradient-to-b from-transparent via-white/10 to-transparent" />
        <div className="absolute right-0 top-0 h-full w-px bg-gradient-to-b from-transparent via-white/10 to-transparent" />
      </div>
      <main className="mx-auto max-w-7xl px-6 py-12 md:py-20">
        <section className="grid items-start gap-10 md:grid-cols-[1.1fr,1fr]">
          <div className="space-y-5">
            <div className="inline-flex items-center gap-2 rounded-full border border-white/10 bg-white/5 px-3 py-1 text-xs text-zinc-300">
              <span className="inline-block h-1.5 w-1.5 rounded-full bg-blue-500" />
              Launching soon
            </div>
            <h1 className="text-3xl font-semibold leading-tight sm:text-4xl">
              <span className="text-white">ROK</span> <span className="text-blue-500">BATTLES</span>
            </h1>
            <p className="max-w-xl text-sm leading-relaxed text-zinc-400 sm:text-base">
              A community-driven platform for sharing battle reports and surfacing actionable trends
              in Rise of Kingdoms.
            </p>
            <div className="flex flex-wrap gap-3">
              <a
                href="https://discord.gg/G33SzQgx6d"
                target="_blank"
                rel="noreferrer noopener"
                className="inline-flex items-center gap-2 rounded-xl bg-blue-600 px-4 py-2 transition hover:bg-blue-500 focus-visible:outline-none"
              >
                <DiscordIcon className="h-5 w-5 fill-white" />
                <span className="text-white">Discord</span>
              </a>
              <a
                href="https://github.com/rokbattles/rokbattles"
                target="_blank"
                rel="noreferrer noopener"
                className="inline-flex items-center gap-2 rounded-xl bg-zinc-800 px-4 py-2 transition hover:bg-zinc-700 focus-visible:outline-none"
              >
                <GitHubIcon className="h-5 w-5 fill-white" />
                <span className="text-white">GitHub</span>
              </a>
            </div>
          </div>
          <div className="relative rounded-2xl border border-white/10 bg-white/[0.03] p-4 shadow-sm">
            <div className="mb-3 flex items-center justify-between">
              <span className="text-xs uppercase tracking-wide text-zinc-400">Data Preview</span>
              <div className="flex items-center gap-2">
                <Star className="h-4 w-4 text-zinc-400" />
                <Forward className="h-4 w-4 text-zinc-400" />
              </div>
            </div>
            <div className="grid grid-cols-2 gap-4">
              <div className="space-y-3">
                <div className="h-3 animate-pulse rounded bg-white/10 w-1/2" />
                <div className="flex flex-wrap gap-1.5">
                  {["Gorgo", "Heraclius"].map((x) => (
                    <Badge key={x}>{x}</Badge>
                  ))}
                </div>
                <div className="grid grid-cols-3 gap-2 text-xs text-zinc-300">
                  <Stat label="Dead" value="3,439" />
                  <Stat label="Sev Wnd" value="121k" />
                  <Stat label="KP" value="2.02M" />
                </div>
                <div className="flex flex-wrap gap-2 text-xs">
                  <InscriptionBadge>Reflector</InscriptionBadge>
                  <InscriptionBadge rarity="special">Toppler</InscriptionBadge>
                  <InscriptionBadge rarity="rare">Raider</InscriptionBadge>
                  <InscriptionBadge rarity="rare">Hardheaded</InscriptionBadge>
                </div>
              </div>
              <div className="space-y-3">
                <div className="h-3 animate-pulse rounded bg-white/10 w-1/2" />
                <div className="flex flex-wrap gap-1.5">
                  {["Ashurbanipal", "Zhuge"].map((x) => (
                    <Badge key={x}>{x}</Badge>
                  ))}
                </div>
                <div className="grid grid-cols-3 gap-2 text-xs text-zinc-300">
                  <Stat label="Dead" value="139k" />
                  <Stat label="Sev Wnd" value="0" />
                  <Stat label="KP" value="2.30M" />
                </div>
                <div className="flex flex-wrap gap-2 text-xs">
                  <InscriptionBadge>Counterer</InscriptionBadge>
                  <InscriptionBadge rarity="special">Hunter</InscriptionBadge>
                  <InscriptionBadge rarity="special">Unstoppable</InscriptionBadge>
                  <InscriptionBadge rarity="special">Intrepid</InscriptionBadge>
                  <InscriptionBadge rarity="special">Balanced</InscriptionBadge>
                </div>
              </div>
            </div>
          </div>
        </section>
        <section className="mt-6">
          <div className="grid grid-cols-1 gap-6 md:grid-cols-3">
            <div className="rounded-2xl border border-white/10 bg-white/[0.03] p-5 shadow-sm transition hover:border-white/15 md:col-span-2">
              <h3 className="text-base font-semibold text-white">Real-time battle reports</h3>
              <p className="mt-1 text-sm text-zinc-400">
                Live reports that highlight what actually matters in a fight.
              </p>
              <ul className="mt-2 list-disc space-y-1 pl-5 text-sm text-zinc-400">
                <li>Battle results: kills, deads, wounded, power, KP</li>
                <li>Commanders & skills for both sides</li>
                <li>Inscriptions and armament buffs used</li>
                <li>Favorite and share reports with one tap</li>
              </ul>
            </div>
            <div className="rounded-2xl border border-white/10 bg-white/[0.03] p-5 shadow-sm transition hover:border-white/15">
              <h3 className="text-base font-semibold text-white">Filters</h3>
              <p className="mt-1 text-sm text-zinc-400">Slice reports your way.</p>
              <ul className="mt-2 list-disc space-y-1 pl-5 text-sm text-zinc-400">
                <li>KVK only or Ark only</li>
                <li>Kingdom #</li>
                <li>Commander(s)</li>
                <li>Kills, deads, wounded, severely wounded</li>
              </ul>
            </div>
          </div>
        </section>
      </main>
    </div>
  );
}

function Badge({ children }: React.PropsWithChildren) {
  return (
    <span className="rounded-md border border-white/10 bg-white/5 px-2 py-0.5 text-[11px] text-zinc-300">
      {children}
    </span>
  );
}

function InscriptionBadge({
  children,
  rarity = "common",
}: React.PropsWithChildren<{ rarity?: "common" | "rare" | "special" }>) {
  return (
    <span
      className={cn(
        "select-none rounded-full border border-white/10 bg-white/5 px-2.5 py-1 text-[11px]",
        rarity === "common"
          ? "text-zinc-300"
          : rarity === "rare"
            ? "text-blue-300"
            : "text-yellow-300"
      )}
    >
      {children}
    </span>
  );
}

function Stat({ label, value }: { label: string; value: string }) {
  return (
    <div className="rounded-lg border border-white/10 bg-white/5 p-2">
      <div className="text-[10px] text-zinc-400">{label}</div>
      <div className="text-sm text-white">{value}</div>
    </div>
  );
}

// Retrieved from simple icons
function GitHubIcon(props: React.PropsWithoutRef<SVGProps<SVGSVGElement>>) {
  return (
    <svg
      role="img"
      viewBox="0 0 24 24"
      xmlns="http://www.w3.org/2000/svg"
      aria-hidden="true"
      fill="currentColor"
      {...props}
    >
      <title>GitHub</title>
      <path d="M12 .297c-6.63 0-12 5.373-12 12 0 5.303 3.438 9.8 8.205 11.385.6.113.82-.258.82-.577 0-.285-.01-1.04-.015-2.04-3.338.724-4.042-1.61-4.042-1.61C4.422 18.07 3.633 17.7 3.633 17.7c-1.087-.744.084-.729.084-.729 1.205.084 1.838 1.236 1.838 1.236 1.07 1.835 2.809 1.305 3.495.998.108-.776.417-1.305.76-1.605-2.665-.3-5.466-1.332-5.466-5.93 0-1.31.465-2.38 1.235-3.22-.135-.303-.54-1.523.105-3.176 0 0 1.005-.322 3.3 1.23.96-.267 1.98-.399 3-.405 1.02.006 2.04.138 3 .405 2.28-1.552 3.285-1.23 3.285-1.23.645 1.653.24 2.873.12 3.176.765.84 1.23 1.91 1.23 3.22 0 4.61-2.805 5.625-5.475 5.92.42.36.81 1.096.81 2.22 0 1.606-.015 2.896-.015 3.286 0 .315.21.69.825.57C20.565 22.092 24 17.592 24 12.297c0-6.627-5.373-12-12-12" />
    </svg>
  );
}

// Retrieved from simple icons
function DiscordIcon(props: React.PropsWithoutRef<SVGProps<SVGSVGElement>>) {
  return (
    <svg
      role="img"
      viewBox="0 0 24 24"
      xmlns="http://www.w3.org/2000/svg"
      aria-hidden="true"
      fill="currentColor"
      {...props}
    >
      <title>Discord</title>
      <path d="M20.317 4.3698a19.7913 19.7913 0 00-4.8851-1.5152.0741.0741 0 00-.0785.0371c-.211.3753-.4447.8648-.6083 1.2495-1.8447-.2762-3.68-.2762-5.4868 0-.1636-.3933-.4058-.8742-.6177-1.2495a.077.077 0 00-.0785-.037 19.7363 19.7363 0 00-4.8852 1.515.0699.0699 0 00-.0321.0277C.5334 9.0458-.319 13.5799.0992 18.0578a.0824.0824 0 00.0312.0561c2.0528 1.5076 4.0413 2.4228 5.9929 3.0294a.0777.0777 0 00.0842-.0276c.4616-.6304.8731-1.2952 1.226-1.9942a.076.076 0 00-.0416-.1057c-.6528-.2476-1.2743-.5495-1.8722-.8923a.077.077 0 01-.0076-.1277c.1258-.0943.2517-.1923.3718-.2914a.0743.0743 0 01.0776-.0105c3.9278 1.7933 8.18 1.7933 12.0614 0a.0739.0739 0 01.0785.0095c.1202.099.246.1981.3728.2924a.077.077 0 01-.0066.1276 12.2986 12.2986 0 01-1.873.8914.0766.0766 0 00-.0407.1067c.3604.698.7719 1.3628 1.225 1.9932a.076.076 0 00.0842.0286c1.961-.6067 3.9495-1.5219 6.0023-3.0294a.077.077 0 00.0313-.0552c.5004-5.177-.8382-9.6739-3.5485-13.6604a.061.061 0 00-.0312-.0286zM8.02 15.3312c-1.1825 0-2.1569-1.0857-2.1569-2.419 0-1.3332.9555-2.4189 2.157-2.4189 1.2108 0 2.1757 1.0952 2.1568 2.419 0 1.3332-.9555 2.4189-2.1569 2.4189zm7.9748 0c-1.1825 0-2.1569-1.0857-2.1569-2.419 0-1.3332.9554-2.4189 2.1569-2.4189 1.2108 0 2.1757 1.0952 2.1568 2.419 0 1.3332-.946 2.4189-2.1568 2.4189Z" />
    </svg>
  );
}
