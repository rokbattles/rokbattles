import { Dialog } from "@base-ui/react/dialog";
import { cn } from "cn";
import { ArrowUpRight, Menu, X } from "lucide-react";
import { useEffect, useState } from "react";
import { Button } from "../ui/marketing/button";
import { frame, gutter } from "../ui/marketing/layout";
import { Link } from "../ui/marketing/link";

function Logo() {
  return (
    <img
      src="/assets/marketing/logo.svg"
      alt="ROK Battles"
      width={2104}
      height={556.24}
      className="h-auto w-40 sm:w-44"
    />
  );
}

function SocialLinks() {
  return (
    <>
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
    </>
  );
}

export function MarketingHeader() {
  const [open, setOpen] = useState(false);

  useEffect(() => {
    const desktop = window.matchMedia("(min-width: 768px)");
    const closeOnDesktop = () => {
      if (desktop.matches) setOpen(false);
    };
    desktop.addEventListener("change", closeOnDesktop);
    return () => desktop.removeEventListener("change", closeOnDesktop);
  }, []);

  return (
    <div className="border-b border-white/10">
      <header className={cn(frame, gutter, "flex items-center justify-between gap-4 py-5")}>
        <nav aria-label="Main navigation" className="flex shrink-0 items-center gap-8">
          <Link href="/" aria-label="ROK Battles home" className="shrink-0">
            <Logo />
          </Link>
          <Link href="/docs" className="text-sm! max-md:hidden">
            Docs
          </Link>
          <Link href="/#downloads" className="text-sm! max-md:hidden">
            Download
          </Link>
        </nav>
        <nav aria-label="Community and dashboard" className="hidden items-center gap-2 md:flex">
          <SocialLinks />
          <Button href="/app" size="compact">
            Open dashboard <ArrowUpRight aria-hidden="true" />
          </Button>
        </nav>
        <Dialog.Root open={open} onOpenChange={setOpen}>
          <Dialog.Trigger
            render={<Button variant="plain" size="icon" className="md:hidden" />}
            aria-label="Open navigation"
          >
            <Menu aria-hidden="true" />
          </Dialog.Trigger>
          <Dialog.Portal>
            <Dialog.Backdrop className="fixed inset-0 z-40 bg-black/60 transition-opacity duration-300 data-starting-style:opacity-0 data-ending-style:opacity-0 motion-reduce:transition-none" />
            <Dialog.Popup className="fixed inset-y-0 left-0 z-50 flex w-[calc(100%-1rem)] max-w-80 flex-col overflow-y-auto overscroll-contain border-r border-white/10 bg-zinc-950 p-6 pb-[max(1.5rem,env(safe-area-inset-bottom))] text-white shadow-xl outline-none transition-transform duration-300 ease-in-out data-starting-style:-translate-x-full data-ending-style:-translate-x-full motion-reduce:transition-none">
              <Dialog.Title className="sr-only">Navigation</Dialog.Title>
              <div className="flex shrink-0 items-center justify-between gap-4">
                <Link href="/" aria-label="ROK Battles home" onClick={() => setOpen(false)}>
                  <Logo />
                </Link>
                <Dialog.Close
                  render={<Button variant="plain" size="icon" />}
                  aria-label="Close navigation"
                >
                  <X aria-hidden="true" />
                </Dialog.Close>
              </div>
              <nav
                aria-label="Mobile navigation"
                className="mt-10 mb-8 flex flex-col items-stretch gap-4"
              >
                <Link href="/docs" onClick={() => setOpen(false)}>
                  Docs
                </Link>
                <Link href="/#downloads" onClick={() => setOpen(false)}>
                  Download
                </Link>
                <Button href="/app" onClick={() => setOpen(false)}>
                  Open dashboard <ArrowUpRight aria-hidden="true" />
                </Button>
              </nav>
              <nav
                aria-label="Social links"
                className="mt-auto flex shrink-0 gap-2 border-t border-white/10 pt-6"
              >
                <SocialLinks />
              </nav>
            </Dialog.Popup>
          </Dialog.Portal>
        </Dialog.Root>
      </header>
    </div>
  );
}
