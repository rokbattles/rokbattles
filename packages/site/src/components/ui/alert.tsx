"use client";

import { Dialog as BaseDialog } from "@base-ui/react/dialog";
import { cn } from "cn";
import type React from "react";
import { Text } from "@/components/ui/text";
import { ModalPopup, type ModalProps, ModalRoot } from "./modal";

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

export function Alert({
  size = "md",
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
        <div className="grid min-h-full grid-rows-[1fr_auto_1fr] justify-items-center p-8 sm:grid-rows-[1fr_auto_3fr] sm:p-4">
          <ModalPopup
            className={cn(
              className,
              sizes[size],
              "row-start-2 w-full rounded-2xl p-8 shadow-lg ring-1 sm:rounded-2xl sm:p-6 bg-zinc-900 ring-white/10 forced-colors:outline",
              "transition duration-100 will-change-transform data-starting-style:scale-95 data-starting-style:opacity-0 data-ending-style:opacity-0 ease-out data-ending-style:ease-in"
            )}
          >
            {children}
          </ModalPopup>
        </div>
      </BaseDialog.Viewport>
    </ModalRoot>
  );
}

export function AlertTitle({
  className,
  ...props
}: { className?: string } & Omit<BaseDialog.Title.Props, "as" | "className">) {
  return (
    <BaseDialog.Title
      {...props}
      className={cn(
        className,
        "text-balance text-center font-semibold text-base/6 sm:text-wrap sm:text-left sm:text-sm/6 text-white"
      )}
    />
  );
}

export function AlertDescription({
  className,
  ...props
}: { className?: string } & Omit<BaseDialog.Description.Props, "as" | "className">) {
  return (
    <BaseDialog.Description
      render={<Text />}
      {...props}
      className={cn(className, "mt-2 text-pretty text-center sm:text-left")}
    />
  );
}

export function AlertBody({ className, ...props }: React.ComponentPropsWithoutRef<"div">) {
  return <div {...props} className={cn(className, "mt-4")} />;
}

export function AlertActions({ className, ...props }: React.ComponentPropsWithoutRef<"div">) {
  return (
    <div
      {...props}
      className={cn(
        className,
        "mt-6 flex flex-col-reverse items-center justify-end gap-3 *:w-full sm:mt-4 sm:flex-row sm:*:w-auto"
      )}
    />
  );
}
