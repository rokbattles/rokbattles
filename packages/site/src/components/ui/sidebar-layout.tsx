"use client";

import {
  CloseButton as HeadlessCloseButton,
  Dialog as HeadlessDialog,
  DialogBackdrop as HeadlessDialogBackdrop,
  DialogPanel as HeadlessDialogPanel,
} from "@headlessui/react";
import { cn } from "cn";
import { Menu, X } from "lucide-react";
import type React from "react";
import { useState } from "react";
import { NavbarItem } from "@/components/ui/navbar";

function MobileSidebar({
  open,
  close,
  children,
}: React.PropsWithChildren<{ open: boolean; close: () => void }>) {
  return (
    <HeadlessDialog open={open} onClose={close} className="lg:hidden">
      <HeadlessDialogBackdrop
        transition
        className="fixed inset-0 bg-black/30 transition data-closed:opacity-0 data-enter:duration-300 data-enter:ease-out data-leave:duration-200 data-leave:ease-in"
      />
      <HeadlessDialogPanel
        transition
        className="fixed inset-y-0 w-full max-w-80 p-2 transition duration-300 ease-in-out data-closed:-translate-x-full"
      >
        <div className="flex h-full flex-col rounded-lg bg-white shadow-xs ring-1 ring-zinc-950/5 dark:bg-zinc-900 dark:ring-white/10">
          <div className="-mb-3 px-4 pt-3">
            <HeadlessCloseButton as={NavbarItem} aria-label="Close navigation">
              <X data-slot="icon" aria-hidden="true" />
            </HeadlessCloseButton>
          </div>
          {children}
        </div>
      </HeadlessDialogPanel>
    </HeadlessDialog>
  );
}

export function SidebarLayout({
  navbar,
  sidebar,
  children,
  fullWidth = false,
}: React.PropsWithChildren<{
  navbar: React.ReactNode;
  sidebar: React.ReactNode;
  fullWidth?: boolean;
}>) {
  const [showSidebar, setShowSidebar] = useState(false);

  return (
    <div className="relative isolate flex min-h-svh w-full bg-white max-lg:flex-col lg:bg-zinc-100 dark:bg-zinc-900 dark:lg:bg-zinc-950">
      {/* Sidebar on desktop */}
      <div className="fixed inset-y-0 left-0 w-64 max-lg:hidden">{sidebar}</div>

      {/* Sidebar on mobile */}
      <MobileSidebar open={showSidebar} close={() => setShowSidebar(false)}>
        {sidebar}
      </MobileSidebar>

      {/* Navbar on mobile */}
      <header className="flex items-center px-4 lg:hidden">
        <div className="py-2.5">
          <NavbarItem onClick={() => setShowSidebar(true)} aria-label="Open navigation">
            <Menu data-slot="icon" aria-hidden="true" />
          </NavbarItem>
        </div>
        <div className="min-w-0 flex-1">{navbar}</div>
      </header>

      {/* Content */}
      <main className="flex flex-1 flex-col pb-2 lg:min-w-0 lg:pt-2 lg:pr-2 lg:pl-64">
        <div className="grow p-6 lg:rounded-lg lg:bg-white lg:p-10 lg:shadow-xs lg:ring-1 lg:ring-zinc-950/5 dark:lg:bg-zinc-900 dark:lg:ring-white/10">
          <div className={cn("mx-auto w-full", !fullWidth && "max-w-6xl")}>{children}</div>
        </div>
      </main>
    </div>
  );
}
