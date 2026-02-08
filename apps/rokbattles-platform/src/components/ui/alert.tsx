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
import { Text } from "@/components/ui/text";
import { cn } from "@/lib/cn";

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
} & Omit<HeadlessDialogProps, "as" | "className">) {
  return (
    <HeadlessDialog {...props}>
      <HeadlessDialogBackdrop
        className="fixed inset-0 flex w-screen justify-center overflow-y-auto bg-zinc-950/15 px-2 py-2 transition duration-100 focus:outline-0 data-closed:opacity-0 data-enter:ease-out data-leave:ease-in sm:px-6 sm:py-8 lg:px-8 lg:py-16 dark:bg-zinc-950/50"
        transition
      />

      <div className="fixed inset-0 w-screen overflow-y-auto pt-6 sm:pt-0">
        <div className="grid min-h-full grid-rows-[1fr_auto_1fr] justify-items-center p-8 sm:grid-rows-[1fr_auto_3fr] sm:p-4">
          <HeadlessDialogPanel
            className={cn(
              className,
              sizes[size],
              "row-start-2 w-full rounded-2xl bg-white p-8 shadow-lg ring-1 ring-zinc-950/10 sm:rounded-2xl sm:p-6 dark:bg-zinc-900 dark:ring-white/10 forced-colors:outline",
              "transition duration-100 will-change-transform data-closed:data-enter:scale-95 data-closed:opacity-0 data-enter:ease-out data-leave:ease-in"
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

export function AlertTitle({
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
        "text-balance text-center font-semibold text-base/6 text-zinc-950 sm:text-wrap sm:text-left sm:text-sm/6 dark:text-white"
      )}
    />
  );
}

export function AlertDescription({
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
      className={cn(className, "mt-2 text-pretty text-center sm:text-left")}
    />
  );
}

export function AlertBody({
  className,
  ...props
}: React.ComponentPropsWithoutRef<"div">) {
  return <div {...props} className={cn(className, "mt-4")} />;
}

export function AlertActions({
  className,
  ...props
}: React.ComponentPropsWithoutRef<"div">) {
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
