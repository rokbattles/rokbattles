import clsx from "clsx";
import { ArrowUpRight } from "lucide-react";
import { Button } from "../ui/marketing/button";
import { Heading } from "../ui/marketing/heading";
import { card, gutter } from "../ui/marketing/layout";
import { Link } from "../ui/marketing/link";
import { Subheading } from "../ui/marketing/subheading";
import { Text } from "../ui/marketing/text";

export function Drastc() {
  return (
    <section
      id="drastc"
      aria-labelledby="drastc-heading"
      className="grid scroll-mt-8 border-b border-white/10 lg:grid-cols-2"
    >
      <div className={clsx(gutter, "py-12 sm:py-16")}>
        <Heading id="drastc-heading">
          DRASTC.
          <br />
          <span className="text-orange-400">Beyond trade ratio.</span>
        </Heading>
        <Text className="mt-6 max-w-md">
          DRASTC is a multi-dimensional scoring system that evaluates commander pairings using real
          battle data across damage, rage/skill cycle efficiency, assist/support, sustainability,
          trade efficiency, and consistency.
        </Text>
        <Button
          href="https://buymeacoffee.com/davorrok/introducing-drastc"
          target="_blank"
          rel="noopener noreferrer"
          aria-label="Learn more about DRASTC (opens in a new tab)"
          className="mt-8"
        >
          Learn more <ArrowUpRight aria-hidden="true" />
        </Button>
      </div>
      <div className="grid auto-rows-fr gap-px border-t border-white/10 bg-white/10 sm:grid-cols-2 lg:border-t-0 lg:border-l">
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
              <Link
                href={href}
                target="_blank"
                rel="noopener noreferrer"
                className="mt-4"
                aria-label={`Visit ${name} on Discord (opens in a new tab)`}
              >
                Visit community <ArrowUpRight className="size-4" aria-hidden="true" />
              </Link>
            ) : null}
          </article>
        ))}
      </div>
    </section>
  );
}
