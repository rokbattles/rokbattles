"use client";

import { Dialog } from "@base-ui/react/dialog";
import { X } from "lucide-react";
import type { PropsWithChildren } from "react";
import { MobileSidebarCloseContext } from "./mobile-sidebar-context";
import { ModalPopup, ModalRoot } from "./modal";
import { NavbarItem } from "./navbar";

export function MobileSidebar({
  open,
  close,
  children,
}: PropsWithChildren<{ open: boolean; close: () => void }>) {
  return (
    <ModalRoot open={open} onClose={close}>
      <MobileSidebarCloseContext value={close}>
        <Dialog.Backdrop className="fixed inset-0 bg-black/30 transition duration-300 ease-out data-starting-style:opacity-0 data-ending-style:opacity-0 data-ending-style:duration-200 data-ending-style:ease-in lg:hidden" />
        <ModalPopup
          aria-label="Navigation"
          className="fixed inset-y-0 w-full max-w-80 p-2 transition duration-300 ease-in-out data-starting-style:-translate-x-full data-ending-style:-translate-x-full lg:hidden"
        >
          <div className="flex h-full flex-col rounded-lg shadow-xs ring-1 bg-zinc-900 ring-white/10">
            <div className="-mb-3 px-4 pt-3">
              <Dialog.Close
                render={
                  <NavbarItem aria-label="Close navigation">
                    <X data-slot="icon" aria-hidden="true" />
                  </NavbarItem>
                }
              />
            </div>
            {children}
          </div>
        </ModalPopup>
      </MobileSidebarCloseContext>
    </ModalRoot>
  );
}
