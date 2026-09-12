import clsx from "clsx";
import { ArrowUpRight } from "lucide-react";
import { Button } from "../ui/marketing/button";
import { Heading } from "../ui/marketing/heading";
import { GridMarkers, gutter } from "../ui/marketing/layout";
import { Text } from "../ui/marketing/text";

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

export function ReportOverview() {
  return (
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
          ROK Battles reads your Rise of Kingdoms mailcache directory and turns it into structured
          data. That data powers everything from individual battle reports, performance insights,
          and loot tracking to larger-scale analysis and community tools.
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
  );
}
