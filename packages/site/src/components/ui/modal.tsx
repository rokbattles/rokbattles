"use client";

import { Dialog } from "@base-ui/react/dialog";
import { type ReactNode, useRef } from "react";

export type ModalProps = Omit<Dialog.Root.Props, "onOpenChange" | "children"> & {
  children?: ReactNode;
  onClose: (open: boolean) => void;
};

export function ModalRoot({ onClose, children, ...props }: ModalProps) {
  return (
    <Dialog.Root
      {...props}
      onOpenChange={(open) => {
        if (!open) onClose(false);
      }}
    >
      <Dialog.Portal className="relative z-50">{children}</Dialog.Portal>
    </Dialog.Root>
  );
}

export function ModalPopup(props: Dialog.Popup.Props) {
  const ref = useRef<HTMLDivElement>(null);
  return (
    <Dialog.Popup
      ref={ref}
      initialFocus={() =>
        ref.current?.querySelector<HTMLElement>("[autofocus], [data-autofocus]") ?? ref.current
      }
      {...props}
    />
  );
}
