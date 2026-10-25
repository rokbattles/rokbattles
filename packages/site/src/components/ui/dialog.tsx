"use client";

import { Dialog as BaseDialog } from "@base-ui/react/dialog";
import { cn } from "cn";
import type React from "react";
import { ModalPopup, type ModalProps, ModalRoot } from "./modal";
import { Text } from "./text";

const sizes = {
  xs: "sm:max-w-xs",
  sm: "sm:max-w-sm",
  md: "sm:max-w-md",
  lg: "sm:max-w-lg",
  xl: "sm:max-w-xl",
  "2xl": "sm:max-w-2xl",
  "3xl": "sm:max-w-3xl",
  "4xl": "sm:max-w-4xl",
  "5xl": "sm:max-w-5xl",
};

export function Dialog({
  size = "lg",
  className,
  children,
  ...props
}: {
  size?: keyof typeof sizes;
  className?: string;
  children: React.ReactNode;
} & Omit<ModalProps, "as" | "className">) {
  return (
    <ModalRoot {...props}>
      <BaseDialog.Backdrop className="fixed inset-0 flex w-screen justify-center overflow-y-auto px-2 py-2 transition duration-100 focus:outline-0 data-starting-style:opacity-0 data-ending-style:opacity-0 ease-out data-ending-style:ease-in sm:px-6 sm:py-8 lg:px-8 lg:py-16 bg-zinc-950/50" />

      <BaseDialog.Viewport className="fixed inset-0 w-screen overflow-y-auto pt-6 sm:pt-0">
        <div className="grid min-h-full grid-rows-[1fr_auto] justify-items-center sm:grid-rows-[1fr_auto_3fr] sm:p-4">
          <ModalPopup
            className={cn(
              className,
              sizes[size],
              "row-start-2 w-full min-w-0 rounded-t-3xl p-(--gutter) shadow-lg ring-1 [--gutter:--spacing(8)] sm:mb-auto sm:rounded-2xl bg-zinc-900 ring-white/10 forced-colors:outline",
              "transition duration-100 will-change-transform data-starting-style:translate-y-12 data-ending-style:translate-y-12 data-starting-style:opacity-0 data-ending-style:opacity-0 ease-out data-ending-style:ease-in sm:data-starting-style:scale-95 sm:data-starting-style:translate-y-0 sm:data-ending-style:translate-y-0"
            )}
          >
            {children}
          </ModalPopup>
        </div>
      </BaseDialog.Viewport>
    </ModalRoot>
  );
}

export function DialogTitle({
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

export function DialogDescription({
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

export function DialogBody({ className, ...props }: React.ComponentPropsWithoutRef<"div">) {
  return <div {...props} className={cn(className, "mt-6")} />;
}

export function DialogActions({ className, ...props }: React.ComponentPropsWithoutRef<"div">) {
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
