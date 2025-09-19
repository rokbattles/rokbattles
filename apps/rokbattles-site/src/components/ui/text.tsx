import type React from "react";
import { Link } from "@/components/ui/link";
import { cn } from "@/lib/cn";

export function Text({ className, ...props }: React.ComponentPropsWithoutRef<"p">) {
  return (
    <p
      data-slot="text"
      {...props}
      className={cn(className, "text-base/6 text-zinc-400 sm:text-sm/6")}
    />
  );
}

export function TextLink({ className, ...props }: React.ComponentPropsWithoutRef<typeof Link>) {
  return (
    <Link
      {...props}
      className={cn(
        className,
        "text-white underline decoration-white/50 data-hover:decoration-white"
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
        "rounded-sm border border-white/20 bg-white/5 px-0.5 text-sm font-medium text-white sm:text-[0.8125rem]"
      )}
    />
  );
}
