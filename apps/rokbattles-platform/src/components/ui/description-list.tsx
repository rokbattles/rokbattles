import type React from "react";
import { cn } from "@/lib/cn";

export function DescriptionList({
  className,
  ...props
}: React.ComponentPropsWithoutRef<"dl">) {
  return (
    <dl
      {...props}
      className={cn(
        className,
        "grid grid-cols-1 text-base/6 sm:grid-cols-[min(50%,--spacing(80))_auto] sm:text-sm/6"
      )}
    />
  );
}

export function DescriptionTerm({
  className,
  ...props
}: React.ComponentPropsWithoutRef<"dt">) {
  return (
    <dt
      {...props}
      className={cn(
        className,
        "col-start-1 border-zinc-950/5 border-t pt-3 text-zinc-500 first:border-none sm:border-zinc-950/5 sm:border-t sm:py-3 dark:border-white/5 dark:text-zinc-400 sm:dark:border-white/5"
      )}
    />
  );
}

export function DescriptionDetails({
  className,
  ...props
}: React.ComponentPropsWithoutRef<"dd">) {
  return (
    <dd
      {...props}
      className={cn(
        className,
        "pt-1 pb-3 text-zinc-950 sm:border-zinc-950/5 sm:border-t sm:nth-2:border-none sm:py-3 dark:text-white dark:sm:border-white/5"
      )}
    />
  );
}
