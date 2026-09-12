import { cn } from "cn";
import { ArrowDown, ArrowUpRight } from "lucide-react";
import { Button } from "../ui/marketing/button";
import { Heading } from "../ui/marketing/heading";
import { gutter } from "../ui/marketing/layout";
import { Text } from "../ui/marketing/text";

export function Hero() {
  return (
    <section className="border-b border-white/10 text-center" aria-labelledby="hero-heading">
      <div className={cn(gutter, "py-16 sm:py-20 lg:py-24")}>
        <Heading level={1} size="hero" id="hero-heading">
          Every battle
          <br />
          <span className="text-orange-400">tells a bigger story.</span>
        </Heading>
        <Text className="mx-auto mt-6 max-w-xl">
          ROK Battles turns Rise of Kingdoms battle reports into data you can explore, compare and
          use across a growing set of community tools.
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
  );
}
