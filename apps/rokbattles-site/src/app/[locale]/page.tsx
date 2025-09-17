import { hasLocale } from "next-intl";
import { setRequestLocale } from "next-intl/server";
import { DiscordIcon } from "@/components/icons/DiscordIcon";
import { GitHubIcon } from "@/components/icons/GitHubIcon";
import { Heading } from "@/components/ui/heading";
import { Link } from "@/i18n/navigation";
import { routing } from "@/i18n/routing";

export default async function Page({ params }: PageProps<"/[locale]">) {
  const { locale: rawLocale } = await params;
  const locale = hasLocale(routing.locales, rawLocale) ? rawLocale : routing.defaultLocale;
  setRequestLocale(locale);

  return (
    <div className="relative min-h-dvh bg-zinc-950 text-zinc-100 antialiased">
      <div aria-hidden="true" className="pointer-events-none absolute inset-0 mx-auto max-w-7xl">
        <div className="absolute left-0 top-0 h-full w-px bg-gradient-to-b from-transparent via-white/10 to-transparent" />
        <div className="absolute right-0 top-0 h-full w-px bg-gradient-to-b from-transparent via-white/10 to-transparent" />
      </div>
      <main className="mx-auto grid min-h-dvh max-w-7xl place-items-center px-6 py-12 md:py-20">
        <section className="grid items-start gap-6 text-center">
          <div className="space-y-5">
            <Heading>
              <span className="text-white">ROK</span> <span className="text-blue-500">BATTLES</span>
            </Heading>
            <p className="mx-auto max-w-xl text-sm leading-relaxed text-zinc-400 sm:text-base">
              A community-driven platform for sharing battle reports and surfacing actionable trends
              in Rise of Kingdoms.
            </p>
            <div className="flex flex-wrap justify-center gap-3">
              <a
                href="https://github.com/rokbattles/rokbattles/releases"
                target="_blank"
                rel="noreferrer noopener"
                className="inline-flex items-center gap-2 rounded-xl bg-blue-600 px-4 py-2 text-white transition hover:bg-blue-500 focus-visible:outline-none"
              >
                <span>Download App</span>
              </a>
              <Link
                href="/live"
                className="inline-flex items-center gap-2 rounded-xl bg-zinc-800 px-4 py-2 text-white transition hover:bg-zinc-700 focus-visible:outline-none"
              >
                <span>Live Reports</span>
              </Link>
              <a
                href="https://discord.gg/G33SzQgx6d"
                target="_blank"
                rel="noreferrer noopener"
                className="inline-flex items-center gap-2 rounded-xl bg-zinc-800 px-4 py-2 transition hover:bg-zinc-700 focus-visible:outline-none"
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
            <div className="mt-4 flex flex-wrap justify-center gap-4 text-xs text-zinc-500">
              <Link
                href={{ pathname: "/legal/terms-of-service" }}
                className="transition hover:text-zinc-300"
              >
                Terms of Service
              </Link>
              <span aria-hidden="true" className="text-zinc-700">
                |
              </span>
              <Link
                href={{ pathname: "/legal/privacy-policy" }}
                className="transition hover:text-zinc-300"
              >
                Privacy Policy
              </Link>
              <span aria-hidden="true" className="text-zinc-700">
                |
              </span>
              <Link
                href={{ pathname: "/legal/cookie-policy" }}
                className="transition hover:text-zinc-300"
              >
                Cookie Policy
              </Link>
            </div>
          </div>
        </section>
      </main>
    </div>
  );
}
