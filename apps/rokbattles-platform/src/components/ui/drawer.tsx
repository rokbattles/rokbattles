import {
  Description as HeadlessDescription,
  type DescriptionProps as HeadlessDescriptionProps,
  Dialog as HeadlessDialog,
  DialogBackdrop as HeadlessDialogBackdrop,
  DialogPanel as HeadlessDialogPanel,
  type DialogProps as HeadlessDialogProps,
  DialogTitle as HeadlessDialogTitle,
  type DialogTitleProps as HeadlessDialogTitleProps,
} from "@headlessui/react";
import type React from "react";
import { cn } from "@/lib/cn";
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
    panel: "data-closed:-translate-x-full",
  },
  right: {
    container: "right-0 pl-10 sm:pl-16",
    panel: "data-closed:translate-x-full",
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
} & Omit<HeadlessDialogProps, "as" | "className">) {
  const sideStyles = sides[side];

  return (
    <HeadlessDialog {...props}>
      <HeadlessDialogBackdrop
        className="fixed inset-0 bg-zinc-950/25 transition duration-200 data-closed:opacity-0 data-enter:ease-out data-leave:ease-in dark:bg-zinc-950/50"
        transition
      />

      <div className="fixed inset-0 overflow-hidden">
        <div
          className={cn(
            "pointer-events-none fixed inset-y-0 flex max-w-full",
            sideStyles.container
          )}
        >
          <HeadlessDialogPanel
            className={cn(
              className,
              sizes[size],
              "pointer-events-auto flex h-full min-h-0 w-screen flex-col bg-white p-(--gutter) shadow-lg ring-1 ring-zinc-950/10 [--gutter:--spacing(8)] dark:bg-zinc-900 dark:ring-white/10 forced-colors:outline",
              "transition duration-300 will-change-transform data-enter:ease-out data-leave:ease-in",
              sideStyles.panel
            )}
            transition
          >
            {children}
          </HeadlessDialogPanel>
        </div>
      </div>
    </HeadlessDialog>
  );
}

export function DrawerTitle({
  className,
  ...props
}: { className?: string } & Omit<HeadlessDialogTitleProps, "as" | "className">) {
  return (
    <HeadlessDialogTitle
      {...props}
      className={cn(
        className,
        "text-balance font-semibold text-lg/6 text-zinc-950 sm:text-base/6 dark:text-white"
      )}
    />
  );
}

export function DrawerDescription({
  className,
  ...props
}: { className?: string } & Omit<HeadlessDescriptionProps<typeof Text>, "as" | "className">) {
  return <HeadlessDescription as={Text} {...props} className={cn(className, "mt-2 text-pretty")} />;
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
