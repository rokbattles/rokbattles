"use client";

import { ArrowUpRight, Heart } from "lucide-react";
import Image from "next/image";
import { useExtracted } from "next-intl";
import { useState } from "react";
import { Dialog, DialogBody, DialogDescription, DialogTitle } from "@/components/ui/dialog";
import { Link } from "@/components/ui/link";
import { SidebarItem, SidebarLabel } from "@/components/ui/sidebar";

export function DonateDialog() {
  const t = useExtracted();
  const [isOpen, setIsOpen] = useState(false);

  return (
    <>
      <SidebarItem onClick={() => setIsOpen(true)} aria-haspopup="dialog" aria-expanded={isOpen}>
        <Heart data-slot="icon" aria-hidden="true" />
        <SidebarLabel>{t("Donate")}</SidebarLabel>
      </SidebarItem>
      <Dialog open={isOpen} onClose={setIsOpen} size="lg" className="rounded-t-md! sm:rounded-md!">
        <DialogTitle className="text-2xl/8! tracking-tight">
          {t("Keep ROK Battles free for everyone.")}
        </DialogTitle>
        <DialogDescription className="mt-3!">
          {t(
            "Your support helps cover server costs and build better tools for the Rise of Kingdoms community."
          )}
        </DialogDescription>
        <DialogBody className="grid gap-3">
          <Link
            href="https://github.com/sponsors/rokbattles"
            target="_blank"
            rel="noopener noreferrer"
            prefetch={false}
            aria-label={t("{provider} (opens in a new tab)", { provider: "GitHub Sponsors" })}
            className="group flex items-center gap-3 rounded-md border border-white/10 bg-white/5 p-3 transition-colors hover:border-white/25 hover:bg-white/10 focus-visible:outline-2 focus-visible:outline-offset-4 focus-visible:outline-orange-400 motion-reduce:transition-none"
          >
            <span className="flex size-9 shrink-0 items-center justify-center rounded-md bg-white">
              <Image src="/assets/github.svg" alt="" width={20} height={20} aria-hidden="true" />
            </span>
            <span className="min-w-0 flex-1 font-semibold text-white">GitHub Sponsors</span>
            <ArrowUpRight
              className="size-5 shrink-0 text-zinc-500 group-hover:text-white group-focus-visible:text-white"
              aria-hidden="true"
            />
          </Link>
          <Link
            href="https://www.patreon.com/cw/ROKBattles"
            target="_blank"
            rel="noopener noreferrer"
            prefetch={false}
            aria-label={t("{provider} (opens in a new tab)", { provider: "Patreon" })}
            className="group flex items-center gap-3 rounded-md border border-[#ff424d]/20 bg-[#ff424d]/5 p-3 transition-colors hover:border-[#ff424d]/40 hover:bg-[#ff424d]/10 focus-visible:outline-2 focus-visible:outline-offset-4 focus-visible:outline-orange-400 motion-reduce:transition-none"
          >
            <span className="flex size-9 shrink-0 items-center justify-center rounded-md bg-[#ff424d]">
              <Image src="/assets/patreon.svg" alt="" width={18} height={20} aria-hidden="true" />
            </span>
            <span className="min-w-0 flex-1 font-semibold text-white">Patreon</span>
            <ArrowUpRight
              className="size-5 shrink-0 text-zinc-500 group-hover:text-white group-focus-visible:text-white"
              aria-hidden="true"
            />
          </Link>
        </DialogBody>
        <p className="mt-5 text-center text-sm/5 text-zinc-400">
          {t("Every contribution makes a difference. Thank you!")}
        </p>
      </Dialog>
    </>
  );
}
