import clsx from "clsx";
import {
  ArrowDown,
  ArrowUpRight,
  BookOpen,
  ChevronDown,
  FlaskConical,
  Gamepad2,
  Gem,
  Laptop,
  Map as MapIcon,
  Monitor,
  Smartphone,
  Tablet,
  Terminal,
} from "lucide-react";
import Metadata from "../components/metadata";
import { Button } from "../components/ui/marketing/button";
import {
  Dropdown,
  DropdownButton,
  DropdownItem,
  DropdownMenu,
} from "../components/ui/marketing/dropdown";
import { Heading } from "../components/ui/marketing/heading";
import {
  card,
  frame,
  GridMarkers,
  gutter,
  sectionHeading,
} from "../components/ui/marketing/layout";
import { Link } from "../components/ui/marketing/link";
import { Subheading } from "../components/ui/marketing/subheading";
import { bodyText, Text } from "../components/ui/marketing/text";

const reportVisual = (
  <svg
    viewBox="0 0 480 350"
    fill="none"
    aria-hidden="true"
    focusable="false"
    className="w-full max-w-lg"
  >
    <rect
      x="42"
      y="66"
      width="224"
      height="264"
      rx="6"
      className="fill-zinc-950/30 stroke-white/5"
    />
    <rect
      x="54"
      y="54"
      width="224"
      height="264"
      rx="6"
      className="fill-zinc-950/50 stroke-white/10"
    />
    <rect x="66" y="42" width="224" height="264" rx="6" className="fill-zinc-950 stroke-white/20" />
    <path d="m90 66 12-5 12 5v13l-12 9-12-9Z" className="fill-orange-400/10 stroke-orange-400" />
    <path d="M130 69h80M130 82h48" className="stroke-white/30" strokeWidth="4" />
    <path d="M88 107h180" className="stroke-white/10" />
    <path
      d="M90 129h62M90 156h84M90 183h52M90 210h68M90 237h45M90 264h76"
      className="stroke-white/20"
      strokeWidth="4"
    />
    <path d="M222 129h44M234 156h32" className="stroke-orange-400/70" strokeWidth="4" />
    <path d="M90 285h42" className="stroke-white/10" strokeWidth="3" />
    <rect
      x="224"
      y="178"
      width="212"
      height="144"
      rx="6"
      className="fill-zinc-950 stroke-white/20"
    />
    <path d="M244 199h45" className="stroke-white/30" strokeWidth="3" />
    <circle cx="414" cy="198" r="3" className="fill-orange-400" />
    <path
      d="M244 225h172M244 254h172M244 283h172"
      className="stroke-white/10"
      strokeDasharray="3 5"
    />
    <path
      d="m244 278 25-17 24 6 24-29 25 9 24-25 24 8 26-16v84H244Z"
      className="fill-orange-400/10"
    />
    <path
      d="m244 278 25-17 24 6 24-29 25 9 24-25 24 8 26-16"
      className="stroke-orange-400"
      strokeWidth="2"
      strokeLinejoin="round"
    />
    <circle cx="317" cy="238" r="4" className="fill-zinc-950 stroke-orange-400" strokeWidth="2" />
    <circle cx="416" cy="214" r="4" className="fill-orange-400" />
    <path d="M244 298h172" className="stroke-white/20" />
  </svg>
);

const communityTools = [
  {
    icon: FlaskConical,
    title: "Combat Lab",
    description:
      "Lorem ipsum dolor sit amet, consectetur adipiscing elit, sed do eiusmod tempor incididunt ut labore et dolore magna aliqua.",
  },
  {
    icon: MapIcon,
    title: "Territory Planner",
    description:
      "Lorem ipsum dolor sit amet, consectetur adipiscing elit, sed do eiusmod tempor incididunt ut labore et dolore magna aliqua.",
  },
  {
    icon: Gem,
    title: "Loot Explorer",
    description:
      "Lorem ipsum dolor sit amet, consectetur adipiscing elit, sed do eiusmod tempor incididunt ut labore et dolore magna aliqua.",
  },
];

const releaseVersion = "1.6.1";
const releaseUrl = `https://github.com/rokbattles/rokbattles/releases/download/${releaseVersion}`;
const downloads = [
  {
    name: "Windows",
    icon: Monitor,
    builds: [
      { label: "x64", file: `ROK.Battles_${releaseVersion}_x64-setup.exe` },
      { label: "ARM64", file: `ROK.Battles_${releaseVersion}_arm64-setup.exe` },
    ],
  },
  {
    name: "macOS",
    icon: Laptop,
    builds: [
      { label: "Apple Silicon", file: `ROK.Battles_${releaseVersion}_aarch64.dmg` },
      { label: "Intel", file: `ROK.Battles_${releaseVersion}_x64.dmg` },
    ],
  },
  {
    name: "Linux",
    icon: Terminal,
    builds: [
      { label: "x64 · AppImage", file: `ROK.Battles_${releaseVersion}_amd64.AppImage` },
      { label: "x64 · .deb", file: `ROK.Battles_${releaseVersion}_amd64.deb` },
      { label: "ARM64 · AppImage", file: `ROK.Battles_${releaseVersion}_aarch64.AppImage` },
      { label: "ARM64 · .deb", file: `ROK.Battles_${releaseVersion}_arm64.deb` },
    ],
  },
  {
    name: "iOS",
    icon: Tablet,
  },
  {
    name: "Android",
    icon: Smartphone,
  },
  {
    name: "SteamOS",
    icon: Gamepad2,
  },
];

export default function IndexRoute() {
  return (
    <div className="min-h-svh bg-zinc-950 text-white selection:bg-orange-400 selection:text-zinc-950">
      <Metadata />
      <a
        href="#main"
        className="sr-only focus:fixed focus:top-4 focus:left-4 focus:z-50 focus:m-0 focus:h-auto focus:w-auto focus:overflow-visible focus:rounded-sm focus:bg-zinc-900 focus:px-4 focus:py-3 focus:[clip:auto] focus-visible:outline-2 focus-visible:outline-offset-4 focus-visible:outline-orange-400"
      >
        Skip to content
      </a>
      <div className="border-b border-white/10">
        <header
          className={clsx(
            frame,
            gutter,
            "flex flex-col gap-4 py-5 sm:flex-row sm:items-center sm:justify-between"
          )}
        >
          <Link href="/" aria-label="ROK Battles home" className="self-start shrink-0">
            <img
              src="/rokbattles-logo.svg"
              alt="ROK Battles"
              width={2104}
              height={556.24}
              className="h-auto w-40 sm:w-44"
            />
          </Link>
          <nav
            aria-label="Community and dashboard"
            className="flex w-full flex-wrap items-center gap-2 sm:w-auto sm:justify-end"
          >
            <Button variant="plain" size="icon" href="https://github.com/rokbattles/rokbattles">
              <span className="sr-only">ROK Battles on GitHub</span>
              <svg viewBox="0 0 24 24" className="fill-current" aria-hidden="true">
                <path d="M12 .75a11.25 11.25 0 0 0-3.558 21.923c.563.104.769-.244.769-.542 0-.267-.01-.975-.016-1.914-3.13.68-3.79-1.51-3.79-1.51-.512-1.3-1.25-1.646-1.25-1.646-1.022-.699.078-.685.078-.685 1.13.08 1.725 1.16 1.725 1.16 1.005 1.722 2.637 1.225 3.28.937.102-.728.393-1.225.715-1.507-2.499-.284-5.126-1.25-5.126-5.563 0-1.23.44-2.234 1.16-3.021-.117-.285-.503-1.43.11-2.979 0 0 .945-.302 3.094 1.154A10.8 10.8 0 0 1 12 6.18c.957.004 1.92.13 2.82.38 2.148-1.456 3.092-1.154 3.092-1.154.614 1.55.228 2.694.112 2.979.72.787 1.158 1.791 1.158 3.021 0 4.324-2.631 5.276-5.138 5.555.404.35.764 1.042.764 2.1 0 1.517-.014 2.741-.014 3.113 0 .3.203.65.774.54A11.25 11.25 0 0 0 12 .75Z" />
              </svg>
            </Button>
            <Button variant="plain" size="icon" href="https://discord.gg/G33SzQgx6d">
              <span className="sr-only">Join ROK Battles on Discord</span>
              <svg viewBox="0 0 24 24" className="fill-current" aria-hidden="true">
                <path d="M20.317 4.37a19.79 19.79 0 0 0-4.885-1.515.074.074 0 0 0-.079.037c-.211.375-.445.865-.609 1.25a18.27 18.27 0 0 0-5.487 0 12.64 12.64 0 0 0-.618-1.25.077.077 0 0 0-.079-.037A19.736 19.736 0 0 0 3.675 4.37a.07.07 0 0 0-.032.027C.533 9.043-.32 13.579.099 18.057a.082.082 0 0 0 .031.057 19.9 19.9 0 0 0 5.993 3.03.078.078 0 0 0 .084-.028c.462-.63.874-1.295 1.226-1.994a.076.076 0 0 0-.041-.106 13.1 13.1 0 0 1-1.872-.892.077.077 0 0 1-.008-.128c.126-.094.252-.192.372-.291a.074.074 0 0 1 .077-.01c3.928 1.793 8.18 1.793 12.062 0a.074.074 0 0 1 .078.01c.12.099.246.198.373.292a.077.077 0 0 1-.006.127 12.3 12.3 0 0 1-1.873.891.077.077 0 0 0-.04.107c.36.698.772 1.362 1.225 1.993a.076.076 0 0 0 .084.028 19.84 19.84 0 0 0 6.002-3.03.077.077 0 0 0 .031-.056c.5-5.177-.838-9.676-3.548-13.66a.061.061 0 0 0-.031-.03ZM8.02 15.33c-1.183 0-2.157-1.085-2.157-2.419 0-1.333.956-2.419 2.157-2.419 1.21 0 2.176 1.096 2.157 2.42 0 1.333-.956 2.418-2.157 2.418Zm7.975 0c-1.183 0-2.157-1.085-2.157-2.419 0-1.333.955-2.419 2.157-2.419 1.21 0 2.176 1.096 2.157 2.42 0 1.333-.946 2.418-2.157 2.418Z" />
              </svg>
            </Button>
            <Button href="/app" size="compact">
              Open dashboard <ArrowUpRight aria-hidden="true" />
            </Button>
          </nav>
        </header>
      </div>

      <main id="main" className={frame}>
        <section className="border-b border-white/10 text-center" aria-labelledby="hero-heading">
          <div className={clsx(gutter, "py-16 sm:py-20 lg:py-24")}>
            <Heading level={1} size="hero" id="hero-heading">
              Every battle
              <br />
              <span className="text-orange-400">tells a bigger story</span>
            </Heading>
            <Text className="mx-auto mt-6 max-w-xl">
              ROK Battles turns Rise of Kingdoms battle reports into data you can explore, compare
              and use across a growing set of community tools.
            </Text>
            <div className="mt-8 flex flex-wrap justify-center gap-3">
              <Button href="/app" variant="primary">
                Explore battles <ArrowUpRight aria-hidden="true" />
              </Button>
              <Button href="#overview" variant="plain">
                Take a closer look <ArrowDown aria-hidden="true" />
              </Button>
            </div>
          </div>
        </section>

        <section aria-label="Community metrics" className="relative border-b border-white/10">
          <GridMarkers />
          <dl className="grid grid-cols-2 gap-px bg-white/10 sm:grid-cols-4">
            {[
              { value: "40M+", label: "Battle reports" },
              { value: "1.5K+", label: "Users" },
              { value: "X+", label: "Lorem ipsum" },
              { value: "Y+", label: "Lorem ipsum" },
            ].map(({ value, label }) => (
              <div key={value} className={clsx(card, "flex flex-col-reverse gap-3")}>
                <dt className={clsx(bodyText, "text-zinc-400")}>{label}</dt>
                <dd className="text-3xl/none tracking-tight text-orange-400 tabular-nums sm:text-4xl lg:text-5xl">
                  {value}
                </dd>
              </div>
            ))}
          </dl>
        </section>

        <section
          id="overview"
          aria-labelledby="report-manager-heading"
          className="relative grid scroll-mt-8 gap-px border-b border-white/10 bg-white/10 lg:grid-cols-2"
        >
          <GridMarkers />
          <div className="flex items-center justify-center overflow-hidden bg-zinc-900 px-6 py-8 sm:px-8 sm:py-12">
            {reportVisual}
          </div>
          <div
            className={clsx(
              gutter,
              "flex flex-col items-start justify-center bg-zinc-900 py-12 sm:py-16 lg:px-12"
            )}
          >
            <Heading id="report-manager-heading">
              It starts with
              <br />
              <span className="text-orange-400">a battle report.</span>
            </Heading>
            <Text className="mt-6 max-w-md">
              Lorem ipsum dolor sit amet, consectetur adipiscing elit. Sed do eiusmod tempor
              incididunt ut labore et dolore magna aliqua.
            </Text>
            <div className="mt-8 flex flex-wrap gap-3">
              <Button href="/app">
                Explore battles <ArrowUpRight aria-hidden="true" />
              </Button>
              <Button disabled>
                Explore duels <ArrowUpRight aria-hidden="true" />
              </Button>
            </div>
          </div>
        </section>

        <section id="community-tools" aria-label="Community tools" className="relative scroll-mt-8">
          <GridMarkers />
          <div className={sectionHeading}>
            <Heading id="overview-heading">More than battle reports.</Heading>
          </div>
          <div className="grid gap-px border-y border-white/10 bg-white/10 lg:grid-cols-3">
            {communityTools.map(({ icon: Icon, title, description }) => (
              <article key={title} className={card}>
                <div className="mb-8 flex items-center justify-between">
                  <Icon className="size-6 text-orange-400" strokeWidth={1.5} aria-hidden="true" />
                </div>
                <Subheading>{title}</Subheading>
                <Text className="mt-4">{description}</Text>
              </article>
            ))}
          </div>
        </section>

        <section
          id="drastc"
          aria-labelledby="drastc-heading"
          className="grid scroll-mt-8 border-b border-white/10 lg:grid-cols-2"
        >
          <div className={clsx(gutter, "py-12 sm:py-16")}>
            <Heading id="drastc-heading">
              DRASTC.
              <br />
              <span className="text-orange-400">Lorem ipsum dolor.</span>
            </Heading>
            <Text className="mt-6 max-w-md">
              Lorem ipsum dolor sit amet, consectetur adipiscing elit. Sed do eiusmod tempor
              incididunt ut labore et dolore magna aliqua.
            </Text>
            <Text className="mt-4 max-w-md">
              Ut enim ad minim veniam, quis nostrud exercitation ullamco laboris nisi ut aliquip ex
              ea commodo consequat. Duis aute irure dolor in reprehenderit in voluptate velit esse
              cillum dolore eu fugiat nulla pariatur.
            </Text>
            <Button href="https://buymeacoffee.com/davorrok/introducing-drastc" className="mt-8">
              Learn more <ArrowUpRight aria-hidden="true" />
            </Button>
          </div>
          <div className="grid gap-px border-t border-white/10 bg-white/10 sm:grid-cols-2 lg:border-t-0 lg:border-l">
            {[
              {
                name: "Davor",
                role: "Model design",
              },
              {
                name: "ROK Battles",
                role: "Implementation and data",
              },
              {
                name: "The King’s Codex",
                role: "Community collaboration",
                href: "https://discord.gg/kingscodex",
              },
              {
                name: "AQ/HQ",
                role: "Community collaboration",
                href: "https://discord.gg/KP3dsGgsBA",
              },
            ].map(({ name, role, href }) => (
              <article key={name} className={card}>
                <Subheading>{name}</Subheading>
                <Text tone="accent" className="mt-3">
                  {role}
                </Text>
                {href ? (
                  <Link href={href} className="mt-4" aria-label={`Visit ${name} on Discord`}>
                    Visit community <ArrowUpRight className="size-4" aria-hidden="true" />
                  </Link>
                ) : null}
              </article>
            ))}
          </div>
        </section>

        <section
          id="downloads"
          aria-label="Downloads"
          className="relative border-b border-white/10"
        >
          <GridMarkers />
          <div className={sectionHeading}>
            <Heading id="downloads-heading">ROK Battles, wherever you play.</Heading>
          </div>
          <div className="grid gap-px border-t border-white/10 bg-white/10 md:grid-cols-2 lg:grid-cols-3">
            {downloads.map(({ name, icon: Icon, builds }) => (
              <article key={name} className={clsx(card, "flex min-w-0 flex-col")}>
                <Icon
                  className="mb-8 size-6 text-orange-400"
                  strokeWidth={1.5}
                  aria-hidden="true"
                />
                <Subheading>{name}</Subheading>
                <div className="mt-8 flex items-stretch gap-2">
                  {builds ? (
                    <Dropdown>
                      <DropdownButton render={<Button className="w-full" />}>
                        Download for {name} <ChevronDown aria-hidden="true" />
                      </DropdownButton>
                      <DropdownMenu align="end" aria-label={`${name} builds`}>
                        {builds.map(({ label, file }) => (
                          <DropdownItem key={file} href={`${releaseUrl}/${file}`}>
                            {label}
                          </DropdownItem>
                        ))}
                      </DropdownMenu>
                    </Dropdown>
                  ) : (
                    <Button type="button" disabled className="w-full">
                      {name} install guide <BookOpen aria-hidden="true" />
                    </Button>
                  )}
                </div>
              </article>
            ))}
          </div>
        </section>

        <section
          id="community"
          aria-labelledby="community-heading"
          className={clsx(
            gutter,
            "relative border-b border-white/10 py-16 text-center sm:py-20 lg:py-24"
          )}
        >
          <GridMarkers />
          <Heading id="community-heading" size="display">
            Lorem ipsum dolor.
            <br />
            <span className="text-zinc-400">Consectetur adipiscing elit.</span>
          </Heading>
          <Text className="mx-auto mt-6 max-w-md">
            Lorem ipsum dolor sit amet, consectetur adipiscing elit, sed do eiusmod tempor
            incididunt ut labore et dolore magna aliqua.
          </Text>
          <Button href="/app" variant="primary" className="mt-8">
            Explore battles <ArrowUpRight aria-hidden="true" />
          </Button>
        </section>
        <footer className={clsx(gutter, "flex flex-wrap items-center justify-between gap-6 py-8")}>
          <Link href="/" className="shrink-0">
            <img
              src="/rokbattles-logo.svg"
              alt="ROK Battles"
              width={2104}
              height={556.24}
              className="h-auto w-36"
            />
          </Link>
          <div className="flex flex-wrap items-center gap-x-6 gap-y-1">
            {["Terms of Service", "Privacy Policy", "Cookie Settings"].map((label) => (
              <Link key={label} href="#">
                {label}
              </Link>
            ))}
          </div>
        </footer>
      </main>
    </div>
  );
}
