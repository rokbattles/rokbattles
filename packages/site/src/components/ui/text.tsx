import { cn } from "cn";
import type React from "react";
import { Link } from "./link";

export function Text({ className, ...props }: React.ComponentPropsWithoutRef<"p">) {
  return (
    <p
      data-slot="text"
      {...props}
      className={cn(className, "text-base/6 sm:text-sm/6 text-zinc-400")}
    />
  );
}

export function TextLink({ className, ...props }: React.ComponentPropsWithoutRef<typeof Link>) {
  return (
    <Link
      {...props}
      className={cn(
        className,
        "underline text-white decoration-white/50 data-hover:decoration-white"
      )}
    />
  );
}

export function Strong({ className, ...props }: React.ComponentPropsWithoutRef<"strong">) {
  return <strong {...props} className={cn(className, "font-medium text-white")} />;
}

export function Code({ className, ...props }: React.ComponentPropsWithoutRef<"code">) {
  return (
    <code
      {...props}
      className={cn(
        className,
        "rounded-sm border px-0.5 font-medium text-sm sm:text-[0.8125rem] border-white/20 bg-white/5 text-white"
      )}
    />
  );
}
