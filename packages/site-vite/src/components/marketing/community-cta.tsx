import clsx from "clsx";
import { ArrowUpRight } from "lucide-react";
import { Button } from "../ui/marketing/button";
import { Heading } from "../ui/marketing/heading";
import { GridMarkers, gutter } from "../ui/marketing/layout";
import { Text } from "../ui/marketing/text";

export function CommunityCta() {
  return (
    <section
      id="community"
      aria-labelledby="community-heading"
      className={clsx(
        gutter,
        "relative isolate border-b border-white/10 py-16 text-center sm:py-20 lg:py-24"
      )}
    >
      <GridMarkers />
      <div
        aria-hidden="true"
        className="pointer-events-none absolute inset-0 -z-10 bg-[radial-gradient(circle,rgba(255,255,255,0.14)_1px,transparent_1px)] bg-size-[20px_20px] mask-[radial-gradient(ellipse_at_center,black_20%,transparent_80%)]"
      />
      <Heading id="community-heading" size="display">
        See the bigger story
        <br />
        <span className="text-zinc-400">behind every battle.</span>
      </Heading>
      <Text className="mx-auto mt-6 max-w-lg">
        Dive into millions of battle reports, uncover performance trends, compare commander
        pairings, and explore the community tools built from that data.
      </Text>
      <Button href="/app" variant="primary" className="mt-8">
        Explore battles <ArrowUpRight aria-hidden="true" />
      </Button>
    </section>
  );
}
