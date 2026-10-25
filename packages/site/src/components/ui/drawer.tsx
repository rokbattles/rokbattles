"use client";

import { Dialog as BaseDialog } from "@base-ui/react/dialog";
import { cn } from "cn";
import type React from "react";
import { ModalPopup, type ModalProps, ModalRoot } from "./modal";
import { Text } from "./text";

const sizes = {
  xs: "max-w-xs",
  sm: "max-w-sm",
  md: "max-w-md",
  lg: "max-w-lg",
  xl: "max-w-xl",
  "2xl": "max-w-2xl",
  "3xl": "max-w-3xl",
  "4xl": "max-w-4xl",
  "5xl": "max-w-5xl",
} as const;

const sides = {
  left: {
    container: "left-0 pr-10 sm:pr-16",
    panel: "data-starting-style:-translate-x-full data-ending-style:-translate-x-full",
  },
  right: {
    container: "right-0 pl-10 sm:pl-16",
    panel: "data-starting-style:translate-x-full data-ending-style:translate-x-full",
  },
} as const;

export function Drawer({
  size = "md",
  side = "right",
  className,
  children,
  ...props
}: {
  size?: keyof typeof sizes;
  side?: keyof typeof sides;
  className?: string;
  children: React.ReactNode;
} & Omit<ModalProps, "as" | "className">) {
  const sideStyles = sides[side];

  return (
    <ModalRoot {...props}>
      <BaseDialog.Backdrop className="fixed inset-0 transition duration-200 data-starting-style:opacity-0 data-ending-style:opacity-0 ease-out data-ending-style:ease-in bg-zinc-950/50" />

      <BaseDialog.Viewport className="fixed inset-0 overflow-hidden">
        <div
          className={cn(
            "pointer-events-none fixed inset-y-0 flex max-w-full",
            sideStyles.container
          )}
        >
          <ModalPopup
            className={cn(
              className,
              sizes[size],
              "pointer-events-auto flex h-full min-h-0 w-screen flex-col p-(--gutter) shadow-lg ring-1 [--gutter:--spacing(8)] bg-zinc-900 ring-white/10 forced-colors:outline",
              "transition duration-300 will-change-transform ease-out data-ending-style:ease-in",
              sideStyles.panel
            )}
          >
            {children}
          </ModalPopup>
        </div>
      </BaseDialog.Viewport>
    </ModalRoot>
  );
}

export function DrawerTitle({
  className,
  ...props
}: { className?: string } & Omit<BaseDialog.Title.Props, "as" | "className">) {
  return (
    <BaseDialog.Title
      {...props}
      className={cn(className, "text-balance font-semibold text-lg/6 sm:text-base/6 text-white")}
    />
  );
}

export function DrawerDescription({
  className,
  ...props
}: { className?: string } & Omit<BaseDialog.Description.Props, "as" | "className">) {
  return (
    <BaseDialog.Description
      render={<Text />}
      {...props}
      className={cn(className, "mt-2 text-pretty")}
    />
  );
}

export function DrawerBody({ className, ...props }: React.ComponentPropsWithoutRef<"div">) {
  return <div {...props} className={cn(className, "mt-6 min-h-0 flex-1 overflow-y-auto")} />;
}

export function DrawerActions({ className, ...props }: React.ComponentPropsWithoutRef<"div">) {
  return (
    <div
      {...props}
      className={cn(
        className,
        "mt-8 flex flex-col-reverse items-center justify-end gap-3 *:w-full sm:flex-row sm:*:w-auto"
      )}
    />
  );
}
