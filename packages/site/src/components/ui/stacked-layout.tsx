"use client";

import {
  CloseButton as HeadlessCloseButton,
  Dialog as HeadlessDialog,
  DialogBackdrop as HeadlessDialogBackdrop,
  DialogPanel as HeadlessDialogPanel,
} from "@headlessui/react";
import { Menu, X } from "lucide-react";
import type React from "react";
import { useState } from "react";
import { NavbarItem } from "./navbar";

function MobileSidebar({
  open,
  close,
  children,
}: React.PropsWithChildren<{ open: boolean; close: () => void }>) {
  return (
    <HeadlessDialog className="lg:hidden" onClose={close} open={open}>
      <HeadlessDialogBackdrop
        className="fixed inset-0 bg-black/30 transition data-closed:opacity-0 data-enter:duration-300 data-leave:duration-200 data-enter:ease-out data-leave:ease-in"
        transition
      />
      <HeadlessDialogPanel
        className="fixed inset-y-0 w-full max-w-80 p-2 transition duration-300 ease-in-out data-closed:-translate-x-full"
        transition
      >
        <div className="flex h-full flex-col rounded-lg shadow-xs ring-1 bg-zinc-900 ring-white/10">
          <div className="-mb-3 px-4 pt-3">
            <HeadlessCloseButton aria-label="Close navigation" as={NavbarItem}>
              <X data-slot="icon" aria-hidden="true" />
            </HeadlessCloseButton>
          </div>
          {children}
        </div>
      </HeadlessDialogPanel>
    </HeadlessDialog>
  );
}

export function StackedLayout({
  navbar,
  sidebar,
  children,
}: React.PropsWithChildren<{
  navbar: React.ReactNode;
  sidebar: React.ReactNode;
}>) {
  const [showSidebar, setShowSidebar] = useState(false);

  return (
    <div className="relative isolate flex min-h-svh w-full flex-col bg-zinc-900 lg:bg-zinc-950">
      {/* Sidebar on mobile */}
      <MobileSidebar close={() => setShowSidebar(false)} open={showSidebar}>
        {sidebar}
      </MobileSidebar>

      {/* Navbar */}
      <header className="flex items-center px-4">
        <div className="py-2.5 lg:hidden">
          <NavbarItem aria-label="Open navigation" onClick={() => setShowSidebar(true)}>
            <Menu data-slot="icon" aria-hidden="true" />
          </NavbarItem>
        </div>
        <div className="min-w-0 flex-1">{navbar}</div>
      </header>

      {/* Content */}
      <main className="flex flex-1 flex-col pb-2 lg:px-2">
        <div className="grow p-6 lg:rounded-lg lg:p-10 lg:shadow-xs lg:ring-1 lg:bg-zinc-900 lg:ring-white/10">
          <div className="mx-auto max-w-6xl">{children}</div>
        </div>
      </main>
    </div>
  );
}
