import { Hero } from "@/components/sections/hero/Hero";
import Image from "next/image"
import {ButtonLink, PlainButtonLink} from "@/components/elements/Button";

export default function Page() {
  return (
    <>
      <Hero
        id="hero"
        headline="Understand Every Battle"
        subheadline={
          <p>
            A community platform for exploring detailed battle reports, outcomes, and trends across
            Rise of Kingdoms.
          </p>
        }
        cta={
          <div className="flex items-center gap-4">
            <ButtonLink href="#" size="lg">
              Explore platform
            </ButtonLink>
            <PlainButtonLink href="#" size="lg">
              Download desktop app
            </PlainButtonLink>
          </div>
        }
        demo={
          <>
            <Image
              className="bg-white/75 md:hidden dark:hidden"
              src="/img/screenshots/app-light-900.png"
              alt=""
              width={1920}
              height={900}
            />
            <Image
              className="bg-black/75 not-dark:hidden md:hidden"
              src="/img/screenshots/app-dark-900.png"
              alt=""
              width={1920}
              height={900}
            />
            <Image
              className="bg-white/75 max-md:hidden lg:hidden dark:hidden"
              src="/img/screenshots/app-light-1200.png"
              alt=""
              width={1920}
              height={1200}
            />
            <Image
              className="bg-black/75 not-dark:hidden max-md:hidden lg:hidden"
              src="/img/screenshots/app-dark-1200.png"
              alt=""
              width={1920}
              height={1200}
            />
            <Image
              className="bg-white/75 max-lg:hidden dark:hidden"
              src="/img/screenshots/app-light-1500.png"
              alt=""
              width={1920}
              height={1500}
            />
            <Image
              className="bg-black/75 not-dark:hidden max-lg:hidden"
              src="/img/screenshots/app-dark-1500.png"
              alt=""
              width={1920}
              height={1500}
            />
          </>
        }
      />
    </>
  );
}
