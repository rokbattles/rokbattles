import { Dialog } from "@base-ui/react/dialog";
import { X } from "lucide-react";
import type { PropsWithChildren } from "react";
import { NavbarItem } from "./navbar";
import { NavigationCloseContext } from "./navigation-context";

export function MobileSidebar({
  open,
  close,
  children,
}: PropsWithChildren<{ open: boolean; close: () => void }>) {
  return (
    <Dialog.Root
      open={open}
      onOpenChange={(nextOpen) => {
        if (!nextOpen) close();
      }}
    >
      <Dialog.Portal>
        <Dialog.Backdrop className="absolute inset-0 bg-black/30 transition-opacity duration-300 data-starting-style:opacity-0 data-ending-style:opacity-0 motion-reduce:transition-none lg:hidden" />
        <Dialog.Popup className="fixed inset-y-0 left-0 w-full max-w-80 p-2 outline-hidden transition-transform duration-300 ease-in-out data-starting-style:-translate-x-full data-ending-style:-translate-x-full motion-reduce:transition-none lg:hidden">
          <Dialog.Title className="sr-only">Navigation</Dialog.Title>
          <div className="flex h-full flex-col rounded-lg bg-zinc-900 text-white shadow-xs ring-1 ring-white/10 scheme-dark">
            <div className="-mb-3 px-4 pt-3">
              <Dialog.Close render={<NavbarItem />} aria-label="Close navigation">
                <X aria-hidden="true" />
              </Dialog.Close>
            </div>
            <NavigationCloseContext value={close}>{children}</NavigationCloseContext>
          </div>
        </Dialog.Popup>
      </Dialog.Portal>
    </Dialog.Root>
  );
}
