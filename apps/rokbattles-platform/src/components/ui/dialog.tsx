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
} & Omit<HeadlessDialogProps, "as" | "className">) {
  return (
    <HeadlessDialog {...props}>
      <HeadlessDialogBackdrop
        className="fixed inset-0 flex w-screen justify-center overflow-y-auto bg-zinc-950/25 px-2 py-2 transition duration-100 focus:outline-0 data-closed:opacity-0 data-enter:ease-out data-leave:ease-in sm:px-6 sm:py-8 lg:px-8 lg:py-16 dark:bg-zinc-950/50"
        transition
      />

      <div className="fixed inset-0 w-screen overflow-y-auto pt-6 sm:pt-0">
        <div className="grid min-h-full grid-rows-[1fr_auto] justify-items-center sm:grid-rows-[1fr_auto_3fr] sm:p-4">
          <HeadlessDialogPanel
            className={cn(
              className,
              sizes[size],
              "row-start-2 w-full min-w-0 rounded-t-3xl bg-white p-(--gutter) shadow-lg ring-1 ring-zinc-950/10 [--gutter:--spacing(8)] sm:mb-auto sm:rounded-2xl dark:bg-zinc-900 dark:ring-white/10 forced-colors:outline",
              "transition duration-100 will-change-transform data-closed:translate-y-12 data-closed:opacity-0 data-enter:ease-out data-leave:ease-in sm:data-closed:data-enter:scale-95 sm:data-closed:translate-y-0"
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

export function DialogTitle({
  className,
  ...props
}: { className?: string } & Omit<
  HeadlessDialogTitleProps,
  "as" | "className"
>) {
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

export function DialogDescription({
  className,
  ...props
}: { className?: string } & Omit<
  HeadlessDescriptionProps<typeof Text>,
  "as" | "className"
>) {
  return (
    <HeadlessDescription
      as={Text}
      {...props}
      className={cn(className, "mt-2 text-pretty")}
    />
  );
}

export function DialogBody({
  className,
  ...props
}: React.ComponentPropsWithoutRef<"div">) {
  return <div {...props} className={cn(className, "mt-6")} />;
}

export function DialogActions({
  className,
  ...props
}: React.ComponentPropsWithoutRef<"div">) {
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
