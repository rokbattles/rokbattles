import { cn } from "cn";
import { ArrowUpRight } from "lucide-react";
import { Heading } from "../ui/marketing/heading";
import { card, GridMarkers } from "../ui/marketing/layout";
import { Link } from "../ui/marketing/link";
import { Subheading } from "../ui/marketing/subheading";
import { Text } from "../ui/marketing/text";

export function ProjectSupport() {
  return (
    <section
      id="project-support"
      aria-labelledby="project-support-heading"
      className="relative grid gap-px border-b border-white/10 bg-white/10 md:grid-cols-2"
    >
      <GridMarkers />
      <div className={cn(card, "md:row-span-2")}>
        <Heading id="project-support-heading">Everything, for everyone.</Heading>
        <Text className="mt-6 max-w-md">
          No premium tiers, locked features, or paid upgrades. Everything we build is available to
          the entire Rise of Kingdoms community for free.
        </Text>
      </div>
      <div className={cn(card, "flex flex-col items-start")}>
        <Subheading>Open source</Subheading>
        <Text className="mt-2">ROK Battles is dual-licensed under MIT or Apache 2.0.</Text>
        <Link
          href="https://github.com/rokbattles/rokbattles"
          target="_blank"
          rel="noopener noreferrer"
          aria-label="View source on GitHub (opens in a new tab)"
          className="mt-auto"
        >
          View source <ArrowUpRight aria-hidden="true" />
        </Link>
      </div>
      <div className={cn(card, "flex flex-col items-start")}>
        <Subheading>Support the project</Subheading>
        <Text className="mt-2">Help support ongoing development and infrastructure costs.</Text>
        <Link
          href="https://github.com/sponsors/rokbattles"
          target="_blank"
          rel="noopener noreferrer"
          aria-label="Support ROK Battles through GitHub Sponsors (opens in a new tab)"
          className="mt-auto"
        >
          GitHub Sponsors <ArrowUpRight aria-hidden="true" />
        </Link>
      </div>
    </section>
  );
}
