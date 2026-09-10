import clsx from "clsx";
import { ArrowUpRight, Pause, Play } from "lucide-react";
import { useInView, useReducedMotion } from "motion/react";
import { useEffect, useRef, useState } from "react";
import { GridMarkers, sectionHeading } from "../ui/marketing/layout";
import { Link } from "../ui/marketing/link";

type CreatorTestimonial = {
  name: string;
  initials: string;
  quote: string;
  avatarUrl?: string;
  channelUrl?: string;
};

const creatorTestimonials: CreatorTestimonial[] = [
  {
    name: "Sttuu",
    initials: "S",
    avatarUrl: "/assets/marketing/creators/sttuu.jpg",
    channelUrl: "https://www.youtube.com/@Sttuuu",
    quote:
      "ROK Battles has become one of my go-to tools while playing the game. The account and march analysis is unrivalled, and it’s always improving and getting better. I genuinely can’t recommend it enough.",
  },
  {
    name: "WarDaddyChadski",
    initials: "WC",
    avatarUrl: "/assets/marketing/creators/wardaddychadski.jpg",
    channelUrl: "https://www.youtube.com/@WarDaddyChadski",
    quote:
      "ROK Battles is the greatest tool we have ever seen when it comes to the combat of Rise of Kingdoms. Having the data, analytics, and results all in one place for any march, any commander, gear, armaments, etc, is a dream for those of us to love the game and the battle aspect of RoK.",
  },
  {
    name: "Mr Siege",
    initials: "MS",
    avatarUrl: "/assets/marketing/creators/mrsiege.jpg",
    channelUrl: "https://www.youtube.com/@MrSiegeOfficial",
    quote:
      "ROK Battles is my go-to for analyzing trade quality and overall results of my marches. The seamless setup and clean navigation make it a no-brainer tool for any creator or player.",
  },
  {
    name: "DOM 117",
    initials: "D",
    avatarUrl: "/assets/marketing/creators/dom117.jpg",
    channelUrl: "https://www.youtube.com/@dom117x",
    quote: "We theorycrafted it. We tested it. Now we’re here to prove it. Welcome to ROK Battles.",
  },
];

const rotationInterval = 8_000;

export function CreatorTestimonials() {
  const [activeIndex, setActiveIndex] = useState(0);
  const [autoRotate, setAutoRotate] = useState(true);
  const [hovered, setHovered] = useState(false);
  const sectionRef = useRef<HTMLElement>(null);
  const inView = useInView(sectionRef, { amount: 0.5 });
  const reducedMotion = useReducedMotion();
  const rotating = autoRotate && !reducedMotion && inView && !hovered;

  useEffect(() => {
    if (!rotating) return;
    const timer = window.setInterval(() => {
      setActiveIndex((index) => (index + 1) % creatorTestimonials.length);
    }, rotationInterval);
    return () => window.clearInterval(timer);
  }, [rotating]);

  return (
    <section
      ref={sectionRef}
      id="creators"
      aria-labelledby="creators-heading"
      className="relative scroll-mt-8 border-b border-white/10"
    >
      <GridMarkers />
      <div className={sectionHeading}>
        <h2
          id="creators-heading"
          className="text-balance text-[clamp(1.375rem,3.5vw,3rem)] leading-[1.2] font-medium tracking-tight text-white md:whitespace-nowrap"
        >
          Used by creators in the community.
        </h2>
      </div>
      <div
        role="group"
        aria-label="Creator quotes"
        className="border-t border-white/10 md:flex md:h-120"
        onMouseEnter={() => setHovered(true)}
        onMouseLeave={() => setHovered(false)}
        onFocusCapture={() => setAutoRotate(false)}
      >
        {creatorTestimonials.map(({ name, initials, quote, avatarUrl, channelUrl }, index) => {
          const active = index === activeIndex;
          return (
            <div
              key={name}
              className="min-w-0 overflow-hidden border-b border-white/10 bg-zinc-950 md:flex md:flex-[0_0_5rem] md:transition-[flex-grow] md:duration-550 md:ease-[cubic-bezier(0.22,1,0.36,1)] md:not-first:border-l md:not-first:border-white/10 md:data-[active=true]:grow motion-reduce:transition-none"
              data-active={active}
            >
              <button
                type="button"
                id={`creator-trigger-${index}`}
                aria-expanded={active}
                aria-controls={`creator-quote-${index}`}
                className="flex w-full cursor-pointer items-center gap-4 px-6 py-4 text-zinc-400 transition-colors duration-200 hover:bg-white/3 hover:text-white focus-visible:outline-2 focus-visible:-outline-offset-4 focus-visible:outline-orange-400 aria-expanded:text-orange-400 aria-expanded:shadow-[inset_3px_0_var(--color-orange-400)] md:w-20 md:shrink-0 md:flex-col md:px-0 md:py-6 md:aria-expanded:shadow-[inset_0_3px_var(--color-orange-400)] motion-reduce:transition-none"
                onClick={() => {
                  setActiveIndex(index);
                  setAutoRotate(false);
                }}
              >
                <span className="font-mono text-[0.6875rem] tracking-[0.1em]" aria-hidden="true">
                  0{index + 1}
                </span>
                <span className="text-base font-medium whitespace-nowrap md:[margin:auto_0] md:rotate-180 md:text-lg md:[writing-mode:vertical-rl]">
                  {name}
                </span>
                {avatarUrl ? (
                  <img
                    src={avatarUrl}
                    alt=""
                    width={44}
                    height={44}
                    loading="lazy"
                    className="ml-auto flex size-11 shrink-0 items-center justify-center rounded-full border border-white/15 object-cover md:ml-0"
                  />
                ) : (
                  <span className="ml-auto flex size-11 shrink-0 items-center justify-center rounded-full border border-white/15 object-cover md:ml-0">
                    {initials}
                  </span>
                )}
              </button>
              <figure
                id={`creator-quote-${index}`}
                aria-labelledby={`creator-trigger-${index}`}
                hidden={!active}
                className={clsx(
                  "min-w-0 flex-1 flex-col px-6 pt-4 pb-8 md:p-8 md:[animation-delay:150ms]",
                  active ? "flex motion-safe:animate-creator-reveal" : "hidden"
                )}
              >
                <span
                  aria-hidden="true"
                  className="h-14 shrink-0 font-[Georgia,serif] text-8xl leading-none text-orange-400"
                >
                  “
                </span>
                <blockquote className="mt-4 flex-1 text-[clamp(1.125rem,2vw,1.75rem)] leading-[1.6] tracking-[-0.02em] text-zinc-200">
                  <p>{quote}</p>
                </blockquote>
                <figcaption className="mt-8 flex shrink-0 flex-wrap items-center justify-between gap-x-4 gap-y-2 border-t border-white/10 pt-4 text-sm">
                  <span className="font-medium text-white">{name}</span>
                  {channelUrl ? (
                    <Link
                      href={channelUrl}
                      target="_blank"
                      rel="noopener noreferrer"
                      aria-label={`Visit ${name}'s channel (opens in a new tab)`}
                    >
                      Visit channel <ArrowUpRight aria-hidden="true" />
                    </Link>
                  ) : null}
                </figcaption>
              </figure>
            </div>
          );
        })}
      </div>
      <div className="flex min-h-14 items-center justify-between px-6 md:px-8">
        <span className="font-mono text-xs text-zinc-400" aria-hidden="true">
          0{activeIndex + 1} / 0{creatorTestimonials.length}
        </span>
        {!reducedMotion ? (
          <button
            type="button"
            className="inline-flex min-h-11 cursor-pointer items-center gap-2 px-2 text-xs text-zinc-400 hover:text-white focus-visible:outline-2 focus-visible:-outline-offset-4 focus-visible:outline-orange-400"
            onClick={() => setAutoRotate((value) => !value)}
          >
            {autoRotate ? (
              <Pause size={14} aria-hidden="true" />
            ) : (
              <Play size={14} aria-hidden="true" />
            )}
            {autoRotate ? "Pause rotation" : "Resume rotation"}
          </button>
        ) : null}
      </div>
    </section>
  );
}
